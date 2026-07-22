//! API assembly bootstrap for sdkwork-payment.

use axum::Router;
use sdkwork_payment_service_host::PaymentServiceHost;
use sdkwork_web_bootstrap::ContractFallbackConfig;
use std::sync::Arc;

pub struct ApiAssembly {
    pub router: Router,
}

pub async fn assemble_api_router(host: Arc<PaymentServiceHost>) -> ApiAssembly {
    assemble_business_routes(host).await
}

pub async fn assemble_business_routes(host: Arc<PaymentServiceHost>) -> ApiAssembly {
    let mut router = Router::new();
    router =
        router.merge(sdkwork_routes_payment_app_api::gateway_mount_business(host.clone()).await);
    router = router.merge(sdkwork_routes_payment_backend_api::gateway_mount_business(host).await);
    ApiAssembly { router }
}

pub async fn assemble_backend_business_router(host: Arc<PaymentServiceHost>) -> ApiAssembly {
    ApiAssembly {
        router: sdkwork_routes_payment_backend_api::gateway_mount_business(host).await,
    }
}

pub fn gateway_contract_fallback_config() -> ContractFallbackConfig {
    let app_manifest = sdkwork_routes_payment_app_api::gateway_route_manifest();
    let backend_manifest = sdkwork_routes_payment_backend_api::gateway_route_manifest();

    let mut config = ContractFallbackConfig::from_manifest(&app_manifest);
    config.manifest_paths.extend(
        ContractFallbackConfig::from_manifest(&backend_manifest)
            .manifest_paths
            .into_iter(),
    );
    config
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assembly_contract_fallback_contains_both_api_surfaces() {
        let expected_route_count = sdkwork_routes_payment_app_api::gateway_route_manifest()
            .routes()
            .len()
            + sdkwork_routes_payment_backend_api::gateway_route_manifest()
                .routes()
                .len();
        let config = gateway_contract_fallback_config();
        assert_eq!(expected_route_count, config.manifest_paths.len());
        assert!(config.contains("POST", "/app/v3/api/payments/intents"));
        assert!(config.contains("GET", "/backend/v3/api/payments/certificates"));
    }
}
