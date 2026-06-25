use axum::Router;
use axum::routing::get;

pub fn payment_health_router() -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/ready", get(|| async { "ready" }))
}
