//! Gateway assembly for sdkwork-payment.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_api_router, assemble_app_api_contribution, assemble_backend_business_router,
    assemble_business_routes, assemble_federated_app_api_contribution,
    federated_app_route_manifest, gateway_contract_fallback_config, ApiAssembly,
    BusinessRouterAssembly,
};

pub async fn assemble_api_router_from_env() -> Result<ApiAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    assemble_api_router(host).await
}

pub async fn assemble_app_api_contribution_from_env() -> Result<ApiAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    assemble_app_api_contribution(host).await
}

pub async fn assemble_federated_app_api_contribution_from_env() -> Result<ApiAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    assemble_federated_app_api_contribution(host).await
}

pub async fn assemble_business_routes_from_env() -> Result<BusinessRouterAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    Ok(assemble_business_routes(host).await)
}

pub async fn assemble_backend_business_router_from_env() -> Result<BusinessRouterAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    Ok(assemble_backend_business_router(host).await)
}

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}
