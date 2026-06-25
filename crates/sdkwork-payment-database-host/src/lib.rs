use std::path::PathBuf;
use std::sync::Arc;
use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_lifecycle::{lifecycle_options_from_env, LifecycleOrchestrator};
use sdkwork_database_spi::{DatabaseAssetProvider, DatabaseManifest, DefaultDatabaseModule};
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};

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

pub async fn bootstrap_payment_database_from_env() -> Result<PaymentDatabaseHost, String> {
    let _ = dotenvy::dotenv();
    let config = DatabaseConfig::from_env("PAYMENT")
        .map_err(|error| format!("read payment database config failed: {error}"))?;
    let pool = create_pool_from_config(config)
        .await
        .map_err(|error| format!("create payment database pool failed: {error}"))?;
    let app_root = std::env::var("SDKWORK_PAYMENT_APP_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let module = Arc::new(
        DefaultDatabaseModule::from_app_root(&app_root)
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
    Ok(PaymentDatabaseHost { pool, module })
}
