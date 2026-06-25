use axum::Router;
use sdkwork_router_payment_app_api::build_payment_app_router_with_framework;
use sdkwork_router_payment_backend_api::build_payment_backend_router_with_framework;
use sdkwork_payment_api_server::payment_health_router;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let host = Arc::new(PaymentServiceHost::new().await);
    let app = Router::new()
        .merge(payment_health_router())
        .merge(build_payment_app_router_with_framework(host.clone()).await)
        .merge(build_payment_backend_router_with_framework(host).await)
        .layer(CorsLayer::permissive());
    let addr = std::env::var("PAYMENT_API_BIND").unwrap_or_else(|_| "0.0.0.0:18094".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
