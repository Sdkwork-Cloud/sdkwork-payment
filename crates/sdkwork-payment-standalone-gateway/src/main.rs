use axum::http::HeaderValue;
use sdkwork_payment_gateway_assembly::{assemble_application_router, gateway_contract_fallback_config};
use sdkwork_payment_service_host::PaymentServiceHost;
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

/// C13/H1/C23 修复：API server 生产级启动配置。
///
/// - C13: CORS 由 `PAYMENT_API_CORS_ORIGINS` 环境变量驱动（逗号分隔），默认拒绝跨域，
///        替代 `CorsLayer::permissive()` 的致命安全漏洞。
/// - H1:  接入 graceful shutdown、请求超时（30s）、请求体大小限制（1 MiB）。
/// - C23: 接入 TraceLayer 结构化请求追踪（含 span、URI、状态码、耗时）。
#[tokio::main]
async fn main() {
    // 结构化日志输出，生产环境应配合 OTel exporter（后续 P1 阶段接入）。
    tracing_subscriber::fmt::init();

    let host = Arc::new(PaymentServiceHost::new().await);
    let cors_layer = build_cors_layer_from_env();
    let business = assemble_application_router(host)
        .await
        .router
        .layer(cors_layer)
        .layer(RequestBodyLimitLayer::new(1024 * 1024)) // 1 MiB，支付请求体不会超过
        .layer(TimeoutLayer::new(Duration::from_secs(30))) // 30s 超时，防止慢 SQL 拖垮线程池
        .layer(TraceLayer::new_for_http());

    // C17 修复：接入 contract fallback，为 manifest 内未挂载 handler 的路径返回
    // 501 Problem+json、为完全未知路径返回 404 Problem+json（RFC 9457）。
    let service_config = ServiceRouterConfig::default()
        .with_always_ready()
        .with_contract_fallback(gateway_contract_fallback_config());
    let app = service_router(business, service_config);
    let addr = std::env::var("PAYMENT_API_BIND").unwrap_or_else(|_| "0.0.0.0:18094".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");

    tracing::info!(bind = %addr, "payment api server starting");

    // H1 修复：graceful shutdown，收到 SIGINT/Ctrl+C 后停止接受新连接，
    // 等待在途请求完成（最多 30s），避免 K8s 滚动更新断连。
    let shutdown = async {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }

        tracing::info!("payment api server shutdown signal received, draining in-flight requests");
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await
        .expect("serve");
}

/// 从 `PAYMENT_API_CORS_ORIGINS` 构建白名单 CORS 层。
/// - 未设置或为空：返回最严格 CorsLayer（不允许任何跨域）。
/// - 设置为 `*`：显式记录警告但仍不允许（支付系统严禁 wildcard）。
/// - 设置为逗号分隔的 origin 列表：仅允许这些 origin。
fn build_cors_layer_from_env() -> CorsLayer {
    let raw = std::env::var("PAYMENT_API_CORS_ORIGINS").unwrap_or_default();
    let origins: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .collect();

    if origins.is_empty() {
        tracing::warn!("PAYMENT_API_CORS_ORIGINS not set; CORS will deny all cross-origin requests");
        return CorsLayer::new();
    }

    if origins.iter().any(|origin| *origin == "*") {
        tracing::error!(
            "PAYMENT_API_CORS_ORIGINS contains wildcard '*'; payment APIs MUST NOT allow wildcard CORS. Denying all cross-origin requests."
        );
        return CorsLayer::new();
    }

    let parsed: Vec<HeaderValue> = origins
        .iter()
        .filter_map(|origin| match HeaderValue::try_from(*origin) {
            Ok(value) => Some(value),
            Err(error) => {
                tracing::warn!(origin = *origin, error = %error, "invalid CORS origin skipped");
                None
            }
        })
        .collect();

    if parsed.is_empty() {
        tracing::error!("no valid CORS origins parsed; denying all cross-origin requests");
        return CorsLayer::new();
    }

    tracing::info!(origins = ?origins, "CORS allowlist configured");
    CorsLayer::new()
        .allow_origin(AllowOrigin::list(parsed))
        .allow_credentials(true)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers(tower_http::cors::Any)
}
