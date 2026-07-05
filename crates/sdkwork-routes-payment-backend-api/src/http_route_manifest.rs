//! C17 修复：payment backend-api 的 HTTP route manifest。
//!
//! 遵循 `API_SPEC.md` §4.2.1 与 `WEB_BACKEND_SPEC.md` §4.2/§4.3 的要求，backend-api
//! route crate `MUST` 导出 `backend_route_manifest` 与 `gateway_route_manifest`，
//! 并通过 framework contract 类型 `HttpRoute` 声明 manifest，以便：
//! 1. 框架运行时解析 operationId / rate-limit tier / 公共路径；
//! 2. 框架自动派生 `ContractFallbackConfig`，为 manifest 内未挂载 handler 的路径
//!    返回 501 Problem+json、为完全未知路径返回 404 Problem+json；
//! 3. OpenAPI 物化器生成 owner-only authority 文档。
//!
//! 所有受保护路由统一使用 `RouteAuth::DualToken`（`API_SPEC.md` §4.2.1 规定受保护
//! backend-api `MUST` 使用 `dual-token`，agent 路由可用 `agent-token`，payment
//! backend 暂无 agent 路由）。写操作（POST/PATCH）标记 `idempotent = true`，表示
//! 该路由接受 `Idempotency-Key` / `Sdkwork-Request-Hash` 命令头并参与幂等仓储层
//! 去重。DELETE 与 replay action 不标记 idempotent（HTTP DELETE 本身幂等；
//! replay 是递增 retries 的动作，非幂等）。

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest, RouteAuth};

/// payment backend-api 路由前缀（`API_SPEC.md` §4.2.1 规定 backend-api `MUST`
/// 使用 `/backend/v3/api`）。
pub const BACKEND_API_PREFIX: &str = "/backend/v3/api";

/// payment backend-api 公共路径前缀。仅包含 infra 健康检查路径，业务路径全部受
/// dual-token 保护。
pub fn payment_backend_api_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

const HTTP_ROUTES: &[HttpRoute] = &[
    // === Payment Intent（查询） ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/intents",
        "payments",
        "payments.intents.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/intents/{paymentIntentId}",
        "payments",
        "payments.intents.retrieve",
    ),
    // === Payment Method ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/methods",
        "payments",
        "payments.methods.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/methods",
        "payments",
        "payments.methods.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/payments/methods/{methodKey}",
        "payments",
        "payments.methods.update",
    )
    .with_idempotent(true),
    // === Provider Account ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/provider_accounts",
        "payments",
        "payments.providerAccounts.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/provider_accounts",
        "payments",
        "payments.providerAccounts.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/payments/provider_accounts/{providerAccountId}",
        "payments",
        "payments.providerAccounts.update",
    )
    .with_idempotent(true),
    // === Channel ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/channels",
        "payments",
        "payments.channels.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/channels",
        "payments",
        "payments.channels.create",
    )
    .with_idempotent(true),
    // === Route Rule ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/route_rules",
        "payments",
        "payments.routeRules.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/route_rules",
        "payments",
        "payments.routeRules.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Patch,
        "/backend/v3/api/payments/route_rules/{routeRuleId}",
        "payments",
        "payments.routeRules.update",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Delete,
        "/backend/v3/api/payments/route_rules/{routeRuleId}",
        "payments",
        "payments.routeRules.delete",
    ),
    // === Attempt / Webhook / Reconciliation ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/attempts",
        "payments",
        "payments.attempts.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/webhook_events",
        "payments",
        "payments.webhookEvents.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/webhook_events/{eventId}/replays",
        "payments",
        "payments.webhookEvents.replay",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/backend/v3/api/payments/reconciliation_runs",
        "payments",
        "payments.reconciliationRuns.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/reconciliation_runs",
        "payments",
        "payments.reconciliationRuns.create",
    )
    .with_idempotent(true),
    // === Owner Order Confirmation ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/backend/v3/api/payments/owner-orders/{orderId}/confirmations",
        "payments",
        "payments.ownerOrders.confirmations.create",
    ),
];

/// 构造 payment backend-api 的 route manifest。
pub fn backend_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_declares_all_routes_with_dual_token_auth() {
        let manifest = backend_route_manifest();
        assert!(!manifest.routes().is_empty());
        for route in manifest.routes() {
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
    fn manifest_routes_use_backend_api_prefix() {
        let manifest = backend_route_manifest();
        for route in manifest.routes() {
            assert!(
                route.path.starts_with(BACKEND_API_PREFIX),
                "route {:?} {} must start with backend-api prefix {}",
                route.method,
                route.path,
                BACKEND_API_PREFIX,
            );
        }
    }

    #[test]
    fn manifest_has_no_duplicate_method_path_pairs() {
        let manifest = backend_route_manifest();
        let mut seen = std::collections::HashSet::new();
        for route in manifest.routes() {
            let key = (format!("{:?}", route.method), route.path);
            assert!(
                seen.insert(key),
                "duplicate (method, path) pair in backend-api manifest: {:?} {}",
                route.method,
                route.path,
            );
        }
    }

    #[test]
    fn write_routes_are_marked_idempotent() {
        let manifest = backend_route_manifest();
        let idempotent_write_routes: Vec<_> = manifest
            .routes()
            .iter()
            .filter(|route| {
                route.method == HttpMethod::Post || route.method == HttpMethod::Patch
            })
            .filter(|route| route.idempotent)
            .map(|route| route.operation_id)
            .collect();
        // 核心写操作必须标记幂等
        assert!(idempotent_write_routes.contains(&"payments.methods.create"));
        assert!(idempotent_write_routes.contains(&"payments.methods.update"));
        assert!(idempotent_write_routes.contains(&"payments.providerAccounts.create"));
        assert!(idempotent_write_routes.contains(&"payments.channels.create"));
        assert!(idempotent_write_routes.contains(&"payments.routeRules.create"));
        assert!(idempotent_write_routes.contains(&"payments.routeRules.update"));
        assert!(idempotent_write_routes.contains(&"payments.reconciliationRuns.create"));
    }

    #[test]
    fn delete_and_replay_are_not_idempotent() {
        let manifest = backend_route_manifest();
        let non_idempotent: Vec<_> = manifest
            .routes()
            .iter()
            .filter(|route| !route.idempotent)
            .map(|route| route.operation_id)
            .collect();
        // DELETE 本身幂等，replay 是递增 retries 的动作，均不使用幂等头
        assert!(non_idempotent.contains(&"payments.routeRules.delete"));
        assert!(non_idempotent.contains(&"payments.webhookEvents.replay"));
    }

    #[test]
    fn manifest_passes_framework_validations() {
        use sdkwork_web_core::WebRequestContextProfile;

        let manifest = backend_route_manifest();
        let profile = WebRequestContextProfile {
            public_path_prefixes: payment_backend_api_public_path_prefixes(),
            ..WebRequestContextProfile::default()
        };
        manifest
            .validate_public_path_prefixes(&profile.public_path_prefixes)
            .expect("public prefixes must not cover protected manifest routes");
        manifest
            .validate_route_auth_for_surfaces(&profile)
            .expect("all backend-api routes must declare dual-token auth");
        manifest
            .validate_no_ambient_context_path_markers(&profile)
            .expect("manifest must not embed ambient tenant/org scoping");
    }
}
