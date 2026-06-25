use sdkwork_database_sqlx::DatabasePool;
use sdkwork_payment_database_host::{bootstrap_payment_database_from_env, PaymentDatabaseHost};

pub struct PaymentServiceHost {
    database: PaymentDatabaseHost,
}

impl PaymentServiceHost {
    pub async fn new() -> Self {
        Self::from_env().await.expect("payment service host bootstrap failed")
    }

    pub async fn from_env() -> Result<Self, String> {
        let database = bootstrap_payment_database_from_env().await?;
        Ok(Self { database })
    }

    pub fn database_pool(&self) -> &DatabasePool {
        self.database.pool()
    }

    pub fn database_module(&self) -> std::sync::Arc<sdkwork_database_spi::DefaultDatabaseModule> {
        self.database.module()
    }
}
