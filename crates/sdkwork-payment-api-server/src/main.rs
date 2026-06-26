use axum::Router;
use sdkwork_payment_gateway_assembly::assemble_application_router;
use sdkwork_payment_service_host::PaymentServiceHost;
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};
use std::sync::Arc;
use tower_http::cors::CorsLayer;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let host = Arc::new(PaymentServiceHost::new().await);
    let business = assemble_application_router(host).await.router
        .layer(CorsLayer::permissive());
    let app = service_router(business, ServiceRouterConfig::default().with_always_ready());
    let addr = std::env::var("PAYMENT_API_BIND").unwrap_or_else(|_| "0.0.0.0:18094".to_owned());
    let listener = tokio::net::TcpListener::bind(&addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
