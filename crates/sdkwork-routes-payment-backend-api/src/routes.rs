use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

use crate::{
    backend_payment_admin_router_with_postgres_pool, backend_payment_admin_router_with_sqlite_pool,
    backend_payment_intent_router_with_postgres_pool, backend_payment_intent_router_with_sqlite_pool,
};

pub fn build_payment_backend_router(host: Arc<PaymentServiceHost>) -> Router {
    match host.database_pool() {
        DatabasePool::Postgres(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_postgres_pool(pool.clone()))
                .merge(backend_payment_intent_router_with_postgres_pool(pool))
        }
        DatabasePool::Sqlite(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_sqlite_pool(pool.clone()))
                .merge(backend_payment_intent_router_with_sqlite_pool(pool))
        }
    }
}

pub async fn build_payment_backend_router_with_framework(host: Arc<PaymentServiceHost>) -> Router {
    build_payment_backend_router(host)
}
