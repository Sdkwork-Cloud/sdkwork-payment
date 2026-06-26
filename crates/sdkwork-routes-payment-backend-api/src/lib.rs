pub mod backend_payment_admin_router;
pub mod backend_payment_intent_router;
pub mod command_headers;
pub mod routes;
pub mod subject;
pub mod web_bootstrap;

pub use backend_payment_admin_router::{
    backend_payment_admin_router_with_postgres_pool, backend_payment_admin_router_with_sqlite_pool,
    build_backend_payment_admin_router, BackendPaymentMethodListQuery,
    CommerceBackendPaymentAdminStore,
};
pub use backend_payment_intent_router::{
    backend_payment_intent_router_with_postgres_pool, backend_payment_intent_router_with_sqlite_pool,
    build_backend_payment_intent_router, CommerceBackendPaymentIntentStore,
};
pub use routes::build_payment_backend_router_with_framework;

use axum::Router;
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

pub async fn gateway_mount(host: Arc<PaymentServiceHost>) -> Router {
    build_payment_backend_router_with_framework(host).await
}
