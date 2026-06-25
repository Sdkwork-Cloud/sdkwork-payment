use axum::Router;
use sdkwork_iam_web_adapter::IamDatabaseWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::WebRequestContextProfile;

pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_database_resolver_from_env().await;
    let layer = WebFrameworkLayer::new(resolver).with_profile(WebRequestContextProfile {
        public_path_prefixes: vec!["/health".to_owned(), "/ready".to_owned()],
        ..WebRequestContextProfile::default()
    });
    with_web_request_context(router, layer)
}
