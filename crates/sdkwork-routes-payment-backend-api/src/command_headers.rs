use axum::http::HeaderMap;
use serde::Serialize;

pub(crate) const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";
pub(crate) const REQUEST_HASH_HEADER: &str = "Sdkwork-Request-Hash";
pub(crate) const REQUEST_NO_HEADER: &str = "Sdkwork-Request-No";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AppWriteCommandHeaders {
    pub idempotency_key: String,
    pub request_hash: String,
    pub request_no: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WriteCommandHeaderError {
    InvalidHeader(&'static str),
}

pub(crate) fn stable_command_request_hash(scope: &str, parts: &[&str]) -> String {
    let mut normalized = vec![scope];
    normalized.extend(parts);
    normalized
        .iter()
        .map(|part| {
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}

pub(crate) fn stable_json_request_hash(
    scope: &str,
    value: &impl Serialize,
) -> Result<String, WriteCommandHeaderError> {
    let value = serde_json::to_value(value).map_err(|_| {
        WriteCommandHeaderError::InvalidHeader(
            "request body could not be canonicalized for request hash validation",
        )
    })?;
    Ok(stable_canonical_json_request_hash(scope, &value))
}

pub(crate) fn stable_canonical_json_request_hash(scope: &str, value: &serde_json::Value) -> String {
    stable_command_request_hash(scope, &[&canonical_json_string(value)])
}

fn canonical_json_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Bool(value) => value.to_string(),
        serde_json::Value::Number(value) => value.to_string(),
        serde_json::Value::String(value) => {
            serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
        }
        serde_json::Value::Array(values) => {
            let items = values
                .iter()
                .map(canonical_json_string)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{items}]")
        }
        serde_json::Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let items = keys
                .into_iter()
                .filter(|key| !values[*key].is_null())
                .map(|key| {
                    format!(
                        "{}:{}",
                        serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_owned()),
                        canonical_json_string(&values[key])
                    )
                })
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{items}}}")
        }
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn validate_write_payload(
    headers: &HeaderMap,
    scope: &str,
    body: &impl Serialize,
    fallback_request_no: impl FnOnce(&str) -> String,
) -> Result<AppWriteCommandHeaders, WriteCommandHeaderError> {
    let expected_hash = stable_json_request_hash(scope, body)?;
    if let Some(request_hash) = optional_text_header(headers, REQUEST_HASH_HEADER) {
        if expected_hash.trim() != request_hash.trim() {
            return Err(WriteCommandHeaderError::InvalidHeader(
                "Sdkwork-Request-Hash does not match the command payload",
            ));
        }
    }
    let idempotency_key = match optional_text_header(headers, IDEMPOTENCY_KEY_HEADER) {
        Some(value) => validate_idempotency_key(value)?,
        None => sdkwork_utils_rust::uuid(),
    };
    let request_no = optional_text_header(headers, REQUEST_NO_HEADER)
        .unwrap_or_else(|| fallback_request_no(&idempotency_key));
    Ok(AppWriteCommandHeaders {
        idempotency_key,
        request_hash: expected_hash,
        request_no,
    })
}

fn validate_idempotency_key(value: String) -> Result<String, WriteCommandHeaderError> {
    let valid_length = (8..=128).contains(&value.len());
    let valid_characters = value.chars().all(|character| {
        character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-')
    });
    if valid_length && valid_characters {
        Ok(value)
    } else {
        Err(WriteCommandHeaderError::InvalidHeader(
            "Idempotency-Key must contain 8 to 128 letters, digits, dots, underscores, colons, or hyphens",
        ))
    }
}

fn optional_text_header(headers: &HeaderMap, name: &'static str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn write_headers_allow_omitted_optional_client_identity() {
        let parsed = validate_write_payload(
            &HeaderMap::new(),
            "scope",
            &serde_json::json!({"orderId":"o-1"}),
            |key| format!("request-{key}"),
        )
        .expect("headers");
        assert!(!parsed.idempotency_key.is_empty());
        assert!(!parsed.request_hash.is_empty());
        assert!(parsed.request_no.starts_with("request-"));
    }

    #[test]
    fn stable_command_request_hash_is_deterministic() {
        let first = stable_command_request_hash("scope", &["100001", "request-1"]);
        let second = stable_command_request_hash("scope", &["100001", "request-1"]);
        assert_eq!(first, second);
        assert!(!first.is_empty());
    }

    #[test]
    fn stable_json_request_hash_matches_struct_and_value_payloads() {
        use serde::Deserialize;

        let body_json = r#"{"methodKey":"wechat_pay","displayName":"WeChat Pay","providerCode":"wechat_pay","status":"active"}"#;
        let value: serde_json::Value = serde_json::from_str(body_json).expect("json");
        let from_value = stable_canonical_json_request_hash("payment-method-upsert", &value);

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct UpsertPaymentMethodBody {
            method_key: Option<String>,
            display_name: Option<String>,
            provider_code: Option<String>,
            status: Option<String>,
            sort_order: Option<i64>,
        }

        let body: UpsertPaymentMethodBody = serde_json::from_str(body_json).expect("body");
        let from_struct = stable_json_request_hash("payment-method-upsert", &body).expect("hash");

        assert_eq!(from_value, from_struct);
    }

    #[test]
    fn ensure_request_hash_matches_rejects_mismatch() {
        let error = validate_write_payload(
            &{
                let mut headers = HeaderMap::new();
                headers.insert(IDEMPOTENCY_KEY_HEADER, HeaderValue::from_static("idem-1"));
                headers.insert(REQUEST_HASH_HEADER, HeaderValue::from_static("wrong"));
                headers
            },
            "scope",
            &serde_json::json!({"orderId":"o-1"}),
            |_| "request-1".to_owned(),
        )
        .expect_err("mismatch");
        assert!(matches!(error, WriteCommandHeaderError::InvalidHeader(_)));
    }

    #[test]
    fn invalid_idempotency_key_is_rejected() {
        let mut headers = HeaderMap::new();
        headers.insert(IDEMPOTENCY_KEY_HEADER, HeaderValue::from_static("short"));
        let error = validate_write_payload(
            &headers,
            "scope",
            &serde_json::json!({"orderId":"o-1"}),
            |_| "request-1".to_owned(),
        )
        .expect_err("invalid key");
        assert!(matches!(error, WriteCommandHeaderError::InvalidHeader(_)));
    }
}
