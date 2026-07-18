//! Gateway assembly for sdkwork-payment.
//! Application bootstrap lives in `bootstrap.rs`; route inventory is in `assembly-manifest.json`.
// SDKWORK-ASSEMBLY-LIB-CUSTOM

mod bootstrap;
mod generated;

pub use bootstrap::{
    assemble_application_business_router, assemble_application_router,
    assemble_backend_business_router, gateway_contract_fallback_config, ApplicationAssembly,
};

pub async fn assemble_application_router_from_env() -> Result<ApplicationAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    Ok(assemble_application_router(host).await)
}

pub async fn assemble_application_business_router_from_env() -> Result<ApplicationAssembly, String>
{
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    Ok(assemble_application_business_router(host).await)
}

pub async fn assemble_backend_business_router_from_env() -> Result<ApplicationAssembly, String> {
    let host =
        std::sync::Arc::new(sdkwork_payment_service_host::PaymentServiceHost::from_env().await?);
    Ok(assemble_backend_business_router(host).await)
}

pub fn assembly_route_count() -> usize {
    generated::ROUTE_CRATE_COUNT
}
