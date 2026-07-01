use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

use crate::{
    backend_payment_admin_router_with_postgres_pool, backend_payment_admin_router_with_sqlite_pool,
    backend_payment_intent_router_with_postgres_pool, backend_payment_intent_router_with_sqlite_pool,
    owner_order_confirmation_router::{
        owner_order_confirmation_router_with_postgres_pool,
        owner_order_confirmation_router_with_sqlite_pool,
    },
};
use crate::web_bootstrap::wrap_router_with_web_framework_from_env;

pub fn build_payment_backend_router(host: Arc<PaymentServiceHost>) -> Router {
    match host.database_pool() {
        DatabasePool::Postgres(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_postgres_pool(pool.clone()))
                .merge(backend_payment_intent_router_with_postgres_pool(pool.clone()))
                .merge(owner_order_confirmation_router_with_postgres_pool(pool))
        }
        DatabasePool::Sqlite(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_sqlite_pool(pool.clone()))
                .merge(backend_payment_intent_router_with_sqlite_pool(pool.clone()))
                .merge(owner_order_confirmation_router_with_sqlite_pool(pool))
        }
    }
}

/// C10 修复：Backend API 必须与 App API 一样接入 sdkwork-web-framework 18 阶段拦截器链，
/// 注入 IamWebRequestContext（含 dual-token 解析、租户隔离、CORS、请求大小限制、限流、审计等），
/// 否则所有 backend handler 的 Extension<IamAppContext> 永远为 None，IAM 边界完全失守。
pub async fn build_payment_backend_router_with_framework(host: Arc<PaymentServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_payment_backend_router(host)).await
}
