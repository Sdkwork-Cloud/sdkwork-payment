use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_lifecycle::{lifecycle_options_from_env, LifecycleOrchestrator};
use sdkwork_database_spi::{
    DatabaseAssetProvider, DatabaseManifest, DefaultDatabaseModule, SpiError,
};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};
use std::path::PathBuf;
use std::sync::Arc;

pub struct PaymentDatabaseHost {
    pool: DatabasePool,
    module: Arc<DefaultDatabaseModule>,
}

impl PaymentDatabaseHost {
    pub fn pool(&self) -> &DatabasePool {
        &self.pool
    }

    pub fn module(&self) -> Arc<DefaultDatabaseModule> {
        self.module.clone()
    }
}

/// Load the payment-owned database assets for a federated application host.
///
/// Hosts register this module in `DatabaseModuleRegistry` and call
/// `RegistryLifecycleOrchestrator::bootstrap_all_from_env()` on their shared
/// connection pool. The framework then honors the payment module's lifecycle
/// manifest and canonical `SDKWORK_DATABASE_*` lifecycle settings without duplicating its
/// schema or seed assets into the integrating application.
pub fn database_module() -> Result<DefaultDatabaseModule, SpiError> {
    let app_root = std::env::var("SDKWORK_PAYMENT_APP_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let raw = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
            std::fs::canonicalize(&raw).unwrap_or(raw)
        });
    DefaultDatabaseModule::from_app_root(&app_root)
}

/// Bootstrap payment assets against an already-created connection pool.
///
/// This is used by embedded hosts that own the shared pool themselves. Most
/// federated applications should instead register [`database_module`] and use
/// `RegistryLifecycleOrchestrator::bootstrap_all_from_env()` once for every
/// capability module.
pub async fn bootstrap_payment_database_with_pool(pool: &DatabasePool) -> Result<(), String> {
    bootstrap_payment_database(pool.clone()).await.map(|_| ())
}

/// Bootstrap payment assets using the PAYMENT database configuration.
pub async fn bootstrap_payment_database_from_env() -> Result<PaymentDatabaseHost, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("PAYMENT")
        .map_err(|error| format!("read payment database config failed: {error}"))?;
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create payment database pool failed: {error}"))?;
    bootstrap_payment_database(pool).await
}

pub async fn bootstrap_payment_database(pool: DatabasePool) -> Result<PaymentDatabaseHost, String> {
    let module = Arc::new(
        database_module()
            .map_err(|error| format!("load payment database module failed: {error}"))?,
    );
    let manifest = DatabaseManifest::from_file(module.manifest_path())
        .map_err(|error| format!("read payment database manifest failed: {error}"))?;
    let options = lifecycle_options_from_env("PAYMENT", &manifest);
    let orchestrator =
        LifecycleOrchestrator::new(pool.clone(), module.clone()).with_applied_by("sdkwork-payment");
    orchestrator.init().await.map_err(|e| format!("{e}"))?;
    if options.auto_migrate {
        orchestrator.migrate().await.map_err(|e| format!("{e}"))?;
    }
    if options.seed_on_boot {
        orchestrator
            .seed(&options.seed_locale, &options.seed_profile)
            .await
            .map_err(|e| format!("{e}"))?;
    }
    Ok(PaymentDatabaseHost { pool, module })
}

#[cfg(test)]
mod tests {
    use super::database_module;
    use sdkwork_database_spi::DatabaseAssetProvider;

    #[test]
    fn database_module_exposes_payment_owned_assets_for_federated_hosts() {
        let module = database_module().expect("payment database module");

        assert_eq!(module.manifest().module_id, "payment");
        assert!(module.seeds_dir().join("seed.manifest.json").is_file());
    }
}
