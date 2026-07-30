use axum::Router;
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_service_host::PaymentServiceHost;
use std::sync::Arc;

use crate::web_bootstrap::wrap_router_with_web_framework_from_env;
use crate::{
    app_payment_intent_router_with_postgres_pool, app_payment_router_with_postgres_pool,
    app_refund_router_with_postgres_pool, payment_webhook_router_deprecated,
};

pub fn build_payment_app_router(host: Arc<PaymentServiceHost>) -> Router {
    let credentials = ProviderCredentialBundle::from_env();
    let registry = Arc::new(PaymentProviderRegistry::from_credentials(
        credentials.clone(),
    ));
    let pool = host
        .database_pool()
        .as_postgres()
        .expect("payment app-api requires an authoritative PostgreSQL pool")
        .clone();
    Router::new()
        .merge(app_payment_router_with_postgres_pool(
            pool.clone(),
            registry.clone(),
            credentials.clone(),
        ))
        .merge(app_payment_intent_router_with_postgres_pool(
            pool.clone(),
            registry.clone(),
            credentials.clone(),
        ))
        .merge(app_refund_router_with_postgres_pool(
            pool,
            registry,
            credentials,
        ))
        .merge(payment_webhook_router_deprecated())
}

pub async fn build_payment_app_router_with_framework(host: Arc<PaymentServiceHost>) -> Router {
    wrap_router_with_web_framework_from_env(build_payment_app_router(host)).await
}
