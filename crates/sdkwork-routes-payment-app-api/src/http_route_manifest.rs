//! C17 修复：payment app-api 的 HTTP route manifest。
//!
//! 遵循 `API_SPEC.md` §4.2.1 与 `WEB_FRAMEWORK_SPEC.md` §2/§7 的要求，route crate
//! `MUST` 通过 framework contract 类型 `HttpRoute` 声明 manifest，以便：
//! 1. 框架运行时解析 operationId / rate-limit tier / 公共路径；
//! 2. 框架自动派生 `ContractFallbackConfig`，为 manifest 内未挂载 handler 的路径
//!    返回 501 Problem+json、为完全未知路径返回 404 Problem+json；
//! 3. OpenAPI 物化器生成 owner-only authority 文档。
//!
//! 所有受保护路由统一使用 `RouteAuth::DualToken`（`API_SPEC.md` §4.2.1 规定受保护
//! app-api `MUST` 使用 `dual-token`）。写操作（POST）标记 `idempotent = true`，
//! 表示该路由接受 `Idempotency-Key` / `Sdkwork-Request-Hash` 命令头并参与幂等
//! 仓储层去重。

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest, RouteAuth};

/// payment app-api 路由前缀（`API_SPEC.md` §4.2.1 规定 app-api `MUST` 使用
/// `/app/v3/api`）。
pub const APP_API_PREFIX: &str = "/app/v3/api";

/// payment app-api 公共路径前缀。仅包含 infra 健康检查路径，业务路径全部受
/// dual-token 保护。
pub fn payment_app_api_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

const HTTP_ROUTES: &[HttpRoute] = &[
    // === Payment Intent ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents",
        "payments",
        "payments.intents.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/intents/{paymentIntentId}",
        "payments",
        "payments.intents.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents/{paymentIntentId}/cancel",
        "payments",
        "payments.intents.cancel",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents/{paymentIntentId}/attempts",
        "payments",
        "payments.intents.attempts.create",
    )
    .with_idempotent(true),
    // === Payment Method / Record / Attempt / Statistics / Status ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/methods",
        "payments",
        "payments.methods.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/records",
        "payments",
        "payments.records.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/records/{paymentId}",
        "payments",
        "payments.records.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/attempts/{paymentAttemptId}",
        "payments",
        "payments.attempts.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/statistics",
        "payments",
        "payments.statistics.fetch",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/status/{paymentId}",
        "payments",
        "payments.status.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/status/out_trade_no/{outTradeNo}",
        "payments",
        "payments.status.retrieveByOutTradeNo",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/orders/{orderId}/payments",
        "payments",
        "payments.orders.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments",
        "payments",
        "payments.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/reconciliations",
        "payments",
        "payments.reconcile",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/{paymentId}/close",
        "payments",
        "payments.close",
    )
    .with_idempotent(true),
    HttpRoute::public(
        HttpMethod::Post,
        // Deprecated 410 shim — live PSP webhooks: order POST /orders/payments/webhooks/{providerCode}
        "/app/v3/api/payments/webhooks/{providerCode}",
        "payments",
        "payments.webhooks.receive",
    ),
    // === Recharge ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/packages",
        "recharges",
        "recharges.packages.fetch",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/settings",
        "recharges",
        "recharges.settings.fetch",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/orders",
        "recharges",
        "recharges.orders.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/recharges/orders",
        "recharges",
        "recharges.orders.submit",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/recharges/orders/{orderId}",
        "recharges",
        "recharges.orders.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/recharges/orders/{orderId}/cancel",
        "recharges",
        "recharges.orders.cancel",
    )
    .with_idempotent(true),
    // === Refund ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/refunds",
        "refunds",
        "refunds.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/refunds",
        "refunds",
        "refunds.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/refunds/{refundId}",
        "refunds",
        "refunds.retrieve",
    ),
];

/// 构造 payment app-api 的 route manifest。
pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_all_routes_with_dual_token_auth() {
        let manifest = app_route_manifest();
        assert!(!manifest.routes().is_empty());
        for route in manifest.routes() {
            if route.path.contains("/payments/webhooks/") {
                assert_eq!(
                    route.auth,
                    RouteAuth::Public,
                    "provider webhook routes must be public"
                );
                continue;
            }
            assert_eq!(
                route.auth,
                RouteAuth::DualToken,
                "route {:?} {} must be dual-token protected",
                route.method,
                route.path,
            );
        }
    }

    #[test]
    fn manifest_routes_use_app_api_prefix() {
        let manifest = app_route_manifest();
        for route in manifest.routes() {
            assert!(
                route.path.starts_with(APP_API_PREFIX),
                "route {:?} {} must start with app-api prefix {}",
                route.method,
                route.path,
                APP_API_PREFIX,
            );
        }
    }

    #[test]
    fn manifest_has_no_duplicate_method_path_pairs() {
        let manifest = app_route_manifest();
        let mut seen = std::collections::HashSet::new();
        for route in manifest.routes() {
            let key = (format!("{:?}", route.method), route.path);
            assert!(
                seen.insert(key),
                "duplicate (method, path) pair in app-api manifest: {:?} {}",
                route.method,
                route.path,
            );
        }
    }

    #[test]
    fn write_routes_are_marked_idempotent() {
        let manifest = app_route_manifest();
        let idempotent_post_routes: Vec<_> = manifest
            .routes()
            .iter()
            .filter(|route| route.method == HttpMethod::Post)
            .filter(|route| route.idempotent)
            .map(|route| route.operation_id)
            .collect();
        // 至少覆盖核心写操作：create payment / intent / refund / recharge
        assert!(idempotent_post_routes.contains(&"payments.create"));
        assert!(idempotent_post_routes.contains(&"payments.intents.create"));
        assert!(idempotent_post_routes.contains(&"refunds.create"));
        assert!(idempotent_post_routes.contains(&"recharges.orders.submit"));
    }

    #[test]
    fn manifest_declares_full_recharge_proxy_surface() {
        // 防止 recharge_proxy_router 实际暴露的路径与 manifest 声明漂移：
        // - GET  /app/v3/api/recharges/packages
        // - GET  /app/v3/api/recharges/settings
        // - GET  /app/v3/api/recharges/orders
        // - POST /app/v3/api/recharges/orders
        // - GET  /app/v3/api/recharges/orders/{orderId}
        // - POST /app/v3/api/recharges/orders/{orderId}/cancel
        let manifest = app_route_manifest();
        let declared: std::collections::HashSet<(String, &str)> = manifest
            .routes()
            .iter()
            .map(|route| (format!("{:?}", route.method), route.path))
            .collect();
        for (method, path) in [
            ("Get", "/app/v3/api/recharges/packages"),
            ("Get", "/app/v3/api/recharges/settings"),
            ("Get", "/app/v3/api/recharges/orders"),
            ("Post", "/app/v3/api/recharges/orders"),
            ("Get", "/app/v3/api/recharges/orders/{orderId}"),
            ("Post", "/app/v3/api/recharges/orders/{orderId}/cancel"),
        ] {
            let owned = (method.to_string(), path);
            assert!(
                declared.contains(&owned),
                "recharge proxy route {:?} {} must be declared in app-api manifest",
                method,
                path,
            );
        }
    }

    #[test]
    fn manifest_passes_framework_validations() {
        use sdkwork_web_core::WebRequestContextProfile;

        let manifest = app_route_manifest();
        let profile = WebRequestContextProfile {
            public_path_prefixes: payment_app_api_public_path_prefixes(),
            ..WebRequestContextProfile::default()
        };
        manifest
            .validate_public_path_prefixes(&profile.public_path_prefixes)
            .expect("public prefixes must not cover protected manifest routes");
        manifest
            .validate_route_auth_for_surfaces(&profile)
            .expect("all app-api routes must declare dual-token auth");
        manifest
            .validate_no_ambient_context_path_markers(&profile)
            .expect("manifest must not embed ambient tenant/org scoping");
    }
}
