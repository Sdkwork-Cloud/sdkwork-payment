//! C17 修复：payment backend-api 的 web framework 装配。
//!
//! 接入 `HttpRouteManifest`，使框架：
//! 1. 运行时按 manifest 解析 operationId / rate-limit tier / 公共路径；
//! 2. 自动派生 `ContractFallbackConfig`，为 manifest 内未挂载 handler 的路径返回
//!    501 Problem+json、为完全未知路径返回 404 Problem+json；
//! 3. 校验 public_path_prefixes 不覆盖受保护路由、RouteAuth 与 surface 匹配、
//!    无 ambient tenant/org path marker。

use axum::Router;
use sdkwork_iam_web_adapter::IamWebRequestContextResolver;
use sdkwork_web_axum::{with_web_request_context, WebFrameworkLayer};
use sdkwork_web_core::WebRequestContextProfile;

use crate::http_route_manifest::{
    backend_route_manifest, payment_backend_api_public_path_prefixes,
};

/// C17 修复：接受外部传入 resolver 的装配入口，便于测试与网关复用。
pub fn wrap_router_with_web_framework(
    resolver: IamWebRequestContextResolver,
    router: Router,
) -> Router {
    let route_manifest = backend_route_manifest();
    route_manifest
        .validate_public_path_prefixes(&payment_backend_api_public_path_prefixes())
        .expect("payment backend-api public prefixes must not cover protected manifest routes");

    let layer = WebFrameworkLayer::new(resolver)
        .with_profile(WebRequestContextProfile {
            public_path_prefixes: payment_backend_api_public_path_prefixes(),
            ..WebRequestContextProfile::default()
        })
        .with_route_manifest(route_manifest);
    with_web_request_context(router, layer)
}

/// 从环境解析 resolver 并装配 framework（生产入口）。
pub async fn wrap_router_with_web_framework_from_env(router: Router) -> Router {
    let resolver = sdkwork_iam_web_adapter::iam_web_request_context_resolver_from_env().await;
    wrap_router_with_web_framework(resolver, router)
}
