use axum::Router;
use sdkwork_database_sqlx::DatabasePool;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

use crate::web_bootstrap::wrap_router_with_web_framework_from_env;
use crate::{
    backend_payment_admin_router_with_postgres_pool, backend_payment_admin_router_with_sqlite_pool,
    backend_payment_integration_router::{
        backend_payment_integration_router_with_postgres_pool,
        backend_payment_integration_router_with_sqlite_pool,
    },
    backend_payment_intent_router_with_postgres_pool,
    backend_payment_intent_router_with_sqlite_pool,
    backend_payment_refund_router_with_postgres_pool,
    backend_payment_refund_router_with_sqlite_pool,
};

pub fn build_payment_backend_router(host: Arc<PaymentServiceHost>) -> Router {
    match host.database_pool() {
        DatabasePool::Postgres(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_postgres_pool(
                    pool.clone(),
                ))
                .merge(backend_payment_intent_router_with_postgres_pool(
                    pool.clone(),
                ))
                .merge(backend_payment_refund_router_with_postgres_pool(
                    pool.clone(),
                ))
                .merge(backend_payment_integration_router_with_postgres_pool(
                    pool.clone(),
                ))
        }
        DatabasePool::Sqlite(pool, _) => {
            let pool = pool.clone();
            Router::new()
                .merge(backend_payment_admin_router_with_sqlite_pool(pool.clone()))
                .merge(backend_payment_intent_router_with_sqlite_pool(pool.clone()))
                .merge(backend_payment_refund_router_with_sqlite_pool(pool.clone()))
                .merge(backend_payment_integration_router_with_sqlite_pool(
                    pool.clone(),
                ))
        }
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use sdkwork_web_core::HttpMethod;
    use tower::ServiceExt;

    #[tokio::test]
    async fn every_manifest_operation_has_a_runtime_handler() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        let router = Router::new()
            .merge(backend_payment_admin_router_with_sqlite_pool(pool.clone()))
            .merge(backend_payment_intent_router_with_sqlite_pool(pool.clone()))
            .merge(backend_payment_refund_router_with_sqlite_pool(pool.clone()))
            .merge(backend_payment_integration_router_with_sqlite_pool(pool));

        for route in crate::http_route_manifest::backend_route_manifest().routes() {
            let method = match route.method {
                HttpMethod::Get => Method::GET,
                HttpMethod::Post => Method::POST,
                HttpMethod::Put => Method::PUT,
                HttpMethod::Patch => Method::PATCH,
                HttpMethod::Delete => Method::DELETE,
            };
            let path = route
                .path
                .split('/')
                .map(|segment| {
                    if segment.starts_with('{') && segment.ends_with('}') {
                        "contract-id"
                    } else {
                        segment
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            let request = Request::builder()
                .method(method)
                .uri(&path)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .expect("request");
            let status = router
                .clone()
                .oneshot(request)
                .await
                .expect("response")
                .status();
            assert_ne!(status, StatusCode::NOT_FOUND, "missing handler for {path}");
            assert_ne!(
                status,
                StatusCode::METHOD_NOT_ALLOWED,
                "missing method handler for {path}",
            );
        }
    }
}

/// C10 修复：Backend API 必须与 App API 一样接入 sdkwork-web-framework 18 阶段拦截器链，
/// 注入 IamWebRequestContext（含 dual-token 解析、租户隔离、CORS、请求大小限制、限流、审计等），
/// 否则所有 backend handler 的 Extension<IamAppContext> 永远为 None，IAM 边界完全失守。
pub async fn build_payment_backend_router_with_framework(host: Arc<PaymentServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_payment_backend_router(host)).await
}
