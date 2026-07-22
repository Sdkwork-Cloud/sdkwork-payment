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
/// manifest and `SDKWORK_PAYMENT_DATABASE_*` overrides without duplicating its
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

async fn bootstrap_payment_database(pool: DatabasePool) -> Result<PaymentDatabaseHost, String> {
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
    use sdkwork_database_config::{DatabaseConfig, DatabaseEngine};
    use sdkwork_database_lifecycle::RegistryLifecycleOrchestrator;
    use sdkwork_database_spi::{DatabaseAssetProvider, DatabaseModuleRegistry};
    use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool};

    fn restore_env(key: &str, previous: Option<String>) {
        match previous {
            Some(value) => std::env::set_var(key, value),
            None => std::env::remove_var(key),
        }
    }

    #[test]
    fn database_module_exposes_payment_owned_assets_for_federated_hosts() {
        let module = database_module().expect("payment database module");

        assert_eq!(module.manifest().module_id, "payment");
        assert!(module.seeds_dir().join("seed.manifest.json").is_file());
    }

    #[tokio::test]
    async fn registry_bootstrap_applies_payment_test_profile_on_shared_pool() {
        let database_path = std::env::temp_dir().join(format!(
            "sdkwork-payment-registry-{}.sqlite",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&database_path);
        let seed_on_boot_key = "SDKWORK_PAYMENT_DATABASE_SEED_ON_BOOT";
        let seed_profile_key = "SDKWORK_PAYMENT_DATABASE_SEED_PROFILE";
        let previous_seed_on_boot = std::env::var(seed_on_boot_key).ok();
        let previous_seed_profile = std::env::var(seed_profile_key).ok();
        std::env::set_var(seed_on_boot_key, "true");
        std::env::set_var(seed_profile_key, "test");

        let result = async {
            let pool = create_pool_from_config(DatabaseConfig {
                engine: DatabaseEngine::Sqlite,
                url: format!("sqlite:{}", database_path.display()),
                ..DatabaseConfig::default()
            })
            .await
            .expect("shared sqlite pool");
            let registry = DatabaseModuleRegistry::builder()
                .register(database_module().expect("payment database module"))
                .expect("register payment database module")
                .build();
            let results = RegistryLifecycleOrchestrator::new(pool.clone(), registry)
                .with_applied_by("payment-database-host-test")
                .bootstrap_all_from_env()
                .await
                .expect("bootstrap payment module through registry");

            assert_eq!(results, vec![("payment".to_owned(), 2, 5)]);
            let DatabasePool::Sqlite(sqlite_pool, _) = &pool else {
                panic!("expected sqlite pool");
            };
            let method_count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM commerce_payment_method WHERE status = 'active'",
            )
            .fetch_one(sqlite_pool)
            .await
            .expect("active payment method count");
            assert_eq!(method_count, 16);

            let recharge_channel_count = sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*)
                FROM commerce_payment_method m
                INNER JOIN commerce_payment_channel c
                    ON c.tenant_id = m.tenant_id
                   AND c.method_id = m.id
                INNER JOIN commerce_payment_provider_account a
                    ON a.id = c.provider_account_id
                   AND a.tenant_id = c.tenant_id
                WHERE m.tenant_id = '100001'
                  AND m.organization_id = '100002'
                  AND m.method_key = 'wechat_pay'
                  AND m.status = 'active'
                  AND m.deleted_at IS NULL
                  AND c.organization_id = '100002'
                  AND c.currency_code = 'CNY'
                  AND c.status = 'active'
                  AND c.deleted_at IS NULL
                  AND a.organization_id = '100002'
                  AND a.status = 'active'
                  AND a.deleted_at IS NULL
                "#,
            )
            .fetch_one(sqlite_pool)
            .await
            .expect("eligible recharge checkout channel count");
            assert_eq!(recharge_channel_count, 1);
            sqlite_pool.close().await;
        }
        .await;

        restore_env(seed_on_boot_key, previous_seed_on_boot);
        restore_env(seed_profile_key, previous_seed_profile);
        let _ = std::fs::remove_file(&database_path);
        result
    }
}
