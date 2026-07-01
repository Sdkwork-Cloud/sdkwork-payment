use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

use crate::{
    app_payment_intent_router_with_postgres_pool, app_payment_intent_router_with_sqlite_pool,
    app_payment_router_with_postgres_pool, app_payment_router_with_sqlite_pool,
    app_recharge_proxy_router,
    app_refund_router_with_postgres_pool, app_refund_router_with_sqlite_pool,
};
use crate::web_bootstrap::wrap_router_with_web_framework_from_env;

fn recharge_proxy_is_enabled() -> bool {
    !std::env::var("SDKWORK_PAYMENT_DISABLE_RECHARGE_PROXY")
        .ok()
        .is_some_and(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "YES"))
}

pub fn build_payment_app_router(host: Arc<PaymentServiceHost>) -> Router {
    match host.database_pool() {
        DatabasePool::Postgres(pool, _) => {
            let pool = pool.clone();
            let mut router = Router::new()
                .merge(app_payment_router_with_postgres_pool(pool.clone()))
                .merge(app_payment_intent_router_with_postgres_pool(pool.clone()))
                .merge(app_refund_router_with_postgres_pool(pool.clone()));
            if recharge_proxy_is_enabled() {
                router = router.merge(app_recharge_proxy_router());
            }
            router
        }
        DatabasePool::Sqlite(pool, _) => {
            let pool = pool.clone();
            let mut router = Router::new()
                .merge(app_payment_router_with_sqlite_pool(pool.clone()))
                .merge(app_payment_intent_router_with_sqlite_pool(pool.clone()))
                .merge(app_refund_router_with_sqlite_pool(pool.clone()));
            if recharge_proxy_is_enabled() {
                router = router.merge(app_recharge_proxy_router());
            }
            router
        }
    }
}

pub async fn build_payment_app_router_with_framework(host: Arc<PaymentServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_payment_app_router(host)).await
}
