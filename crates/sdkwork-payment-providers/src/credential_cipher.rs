use sdkwork_utils_rust::{aes_gcm_decrypt, aes_gcm_encrypt, derive_aes_256_key, sha256_hash};
use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

pub const PAYMENT_CREDENTIAL_ALGORITHM: &str = "aes256gcm-hkdf-v1";
const HKDF_SALT: &[u8] = b"sdkwork.payment.provider-credential.v1";

#[derive(Debug, Clone)]
pub struct CredentialCipherScope<'a> {
    pub tenant_id: &'a str,
    pub provider_account_id: &'a str,
    pub credential_kind: &'a str,
}

#[derive(Debug, Clone)]
pub struct EncryptedPaymentCredential {
    pub ciphertext: String,
    pub encryption_key_id: String,
    pub encryption_algorithm: String,
    pub fingerprint_sha256: String,
}

pub trait PaymentCredentialCipher: Send + Sync {
    fn encrypt(
        &self,
        scope: CredentialCipherScope<'_>,
        plaintext: &str,
    ) -> Result<EncryptedPaymentCredential, String>;

    fn decrypt(
        &self,
        scope: CredentialCipherScope<'_>,
        ciphertext: &str,
        encryption_key_id: &str,
        encryption_algorithm: &str,
    ) -> Result<String, String>;
}

#[derive(Clone)]
pub struct LocalFilePaymentCredentialCipher {
    master_key: Arc<Vec<u8>>,
    key_id: String,
}

impl LocalFilePaymentCredentialCipher {
    pub fn load_or_create_default() -> Result<Self, String> {
        let path = std::env::current_dir()
            .map_err(|_| "payment credential key storage is unavailable".to_owned())?
            .join(".runtime")
            .join("payment")
            .join("credential-master.key");
        Self::load_or_create(path)
    }

    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|_| "payment credential key storage is unavailable".to_owned())?;
        }
        let master_key = match fs::read(path) {
            Ok(value) => value,
            Err(error) if error.kind() == ErrorKind::NotFound => create_master_key(path)?,
            Err(_) => return Err("payment credential key storage is unavailable".to_owned()),
        };
        Self::from_key_material(master_key)
    }

    pub fn from_key_material(master_key: impl Into<Vec<u8>>) -> Result<Self, String> {
        let master_key = master_key.into();
        if master_key.len() < 32 {
            return Err("payment credential master key is invalid".to_owned());
        }
        let key_id = format!("local-{}", &sha256_hash(&master_key)[..16]);
        Ok(Self {
            master_key: Arc::new(master_key),
            key_id,
        })
    }

    pub fn key_path() -> Result<PathBuf, String> {
        Ok(std::env::current_dir()
            .map_err(|_| "payment credential key storage is unavailable".to_owned())?
            .join(".runtime")
            .join("payment")
            .join("credential-master.key"))
    }

    fn derived_key(&self, scope: &CredentialCipherScope<'_>) -> [u8; 32] {
        let info = format!(
            "tenant={};account={};kind={}",
            scope.tenant_id, scope.provider_account_id, scope.credential_kind
        );
        derive_aes_256_key(&self.master_key, HKDF_SALT, info.as_bytes())
    }
}

impl PaymentCredentialCipher for LocalFilePaymentCredentialCipher {
    fn encrypt(
        &self,
        scope: CredentialCipherScope<'_>,
        plaintext: &str,
    ) -> Result<EncryptedPaymentCredential, String> {
        let key = self.derived_key(&scope);
        let ciphertext = aes_gcm_encrypt(&key, plaintext.as_bytes())
            .map_err(|_| "payment credential encryption failed".to_owned())?;
        Ok(EncryptedPaymentCredential {
            ciphertext,
            encryption_key_id: self.key_id.clone(),
            encryption_algorithm: PAYMENT_CREDENTIAL_ALGORITHM.to_owned(),
            fingerprint_sha256: sha256_hash(plaintext.as_bytes()),
        })
    }

    fn decrypt(
        &self,
        scope: CredentialCipherScope<'_>,
        ciphertext: &str,
        encryption_key_id: &str,
        encryption_algorithm: &str,
    ) -> Result<String, String> {
        if encryption_key_id != self.key_id || encryption_algorithm != PAYMENT_CREDENTIAL_ALGORITHM
        {
            return Err("payment credential encryption key is unavailable".to_owned());
        }
        let key = self.derived_key(&scope);
        let plaintext = aes_gcm_decrypt(&key, ciphertext)
            .map_err(|_| "payment credential decryption failed".to_owned())?;
        String::from_utf8(plaintext).map_err(|_| "payment credential payload is invalid".to_owned())
    }
}

static PAYMENT_CREDENTIAL_CIPHER: OnceLock<Arc<dyn PaymentCredentialCipher>> = OnceLock::new();

pub fn install_payment_credential_cipher(
    cipher: Arc<dyn PaymentCredentialCipher>,
) -> Result<(), String> {
    PAYMENT_CREDENTIAL_CIPHER
        .set(cipher)
        .map_err(|_| "payment credential cipher is already initialized".to_owned())
}

pub fn payment_credential_cipher() -> Result<Arc<dyn PaymentCredentialCipher>, String> {
    if let Some(cipher) = PAYMENT_CREDENTIAL_CIPHER.get() {
        return Ok(cipher.clone());
    }
    let cipher: Arc<dyn PaymentCredentialCipher> =
        Arc::new(LocalFilePaymentCredentialCipher::load_or_create_default()?);
    let _ = PAYMENT_CREDENTIAL_CIPHER.set(cipher.clone());
    Ok(PAYMENT_CREDENTIAL_CIPHER.get().cloned().unwrap_or(cipher))
}

fn create_master_key(path: &Path) -> Result<Vec<u8>, String> {
    let value = format!(
        "{}{}{}",
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4(),
        uuid::Uuid::new_v4()
    )
    .into_bytes();
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    match options.open(path) {
        Ok(mut file) => {
            file.write_all(&value)
                .and_then(|_| file.sync_all())
                .map_err(|_| "payment credential key storage is unavailable".to_owned())?;
            Ok(value)
        }
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            fs::read(path).map_err(|_| "payment credential key storage is unavailable".to_owned())
        }
        Err(_) => Err("payment credential key storage is unavailable".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cipher_is_randomized_and_scope_bound() {
        let cipher =
            LocalFilePaymentCredentialCipher::from_key_material(vec![7; 64]).expect("test cipher");
        let scope = CredentialCipherScope {
            tenant_id: "tenant-a",
            provider_account_id: "account-a",
            credential_kind: "merchant_private_key",
        };
        let first = cipher
            .encrypt(scope.clone(), "real-secret")
            .expect("encrypt");
        let second = cipher
            .encrypt(scope.clone(), "real-secret")
            .expect("encrypt");
        assert_ne!(first.ciphertext, second.ciphertext);
        assert!(!first.ciphertext.contains("real-secret"));
        assert_eq!(
            cipher
                .decrypt(
                    scope,
                    &first.ciphertext,
                    &first.encryption_key_id,
                    &first.encryption_algorithm,
                )
                .expect("decrypt"),
            "real-secret"
        );
    }

    #[test]
    fn cipher_rejects_cross_tenant_decryption() {
        let cipher =
            LocalFilePaymentCredentialCipher::from_key_material(vec![8; 64]).expect("test cipher");
        let encrypted = cipher
            .encrypt(
                CredentialCipherScope {
                    tenant_id: "tenant-a",
                    provider_account_id: "account-a",
                    credential_kind: "api_v3_key",
                },
                "real-secret",
            )
            .expect("encrypt");
        let result = cipher.decrypt(
            CredentialCipherScope {
                tenant_id: "tenant-b",
                provider_account_id: "account-a",
                credential_kind: "api_v3_key",
            },
            &encrypted.ciphertext,
            &encrypted.encryption_key_id,
            &encrypted.encryption_algorithm,
        );
        assert!(result.is_err());
    }
}
