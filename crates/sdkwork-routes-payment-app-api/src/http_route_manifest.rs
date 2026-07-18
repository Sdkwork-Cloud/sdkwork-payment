//! C17 淇锛歱ayment app-api 鐨?HTTP route manifest銆?//!
//! 閬靛惊 `API_SPEC.md` 搂4.2.1 涓?`WEB_FRAMEWORK_SPEC.md` 搂2/搂7 鐨勮姹傦紝route crate
//! `MUST` 閫氳繃 framework contract 绫诲瀷 `HttpRoute` 澹版槑 manifest锛屼互渚匡細
//! 1. 妗嗘灦杩愯鏃惰В鏋?operationId / rate-limit tier / 鍏叡璺緞锛?//! 2. 妗嗘灦鑷姩娲剧敓 `ContractFallbackConfig`锛屼负 manifest 鍐呮湭鎸傝浇 handler 鐨勮矾寰?//!    杩斿洖 501 Problem+json銆佷负瀹屽叏鏈煡璺緞杩斿洖 404 Problem+json锛?//! 3. OpenAPI 鐗╁寲鍣ㄧ敓鎴?owner-only authority 鏂囨。銆?//!
//! 鎵€鏈夊彈淇濇姢璺敱缁熶竴浣跨敤 `RouteAuth::DualToken`锛坄API_SPEC.md` 搂4.2.1 瑙勫畾鍙椾繚鎶?//! app-api `MUST` 浣跨敤 `dual-token`锛夈€傚啓鎿嶄綔锛圥OST锛夋爣璁?`idempotent = true`锛?//! 琛ㄧず璇ヨ矾鐢辨帴鍙?`Idempotency-Key` / `Sdkwork-Request-Hash` 鍛戒护澶村苟鍙備笌骞傜瓑
//! 浠撳偍灞傚幓閲嶃€?
use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest};

/// payment app-api 璺敱鍓嶇紑锛坄API_SPEC.md` 搂4.2.1 瑙勫畾 app-api `MUST` 浣跨敤
/// `/app/v3/api`锛夈€?pub const APP_API_PREFIX: &str = "/app/v3/api";

/// payment app-api 鍏叡璺緞鍓嶇紑銆備粎鍖呭惈 infra 鍋ュ悍妫€鏌ヨ矾寰勶紝涓氬姟璺緞鍏ㄩ儴鍙?/// dual-token 淇濇姢銆?pub fn payment_app_api_public_path_prefixes() -> Vec<String> {
    sdkwork_web_bootstrap::infra_public_path_prefixes()
}

const HTTP_ROUTES: &[HttpRoute] = &[
    // === Payment Intent ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents",
        "commerce",
        "payments.intents.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/intents/{paymentIntentId}",
        "commerce",
        "payments.intents.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents/{paymentIntentId}/cancel",
        "commerce",
        "payments.intents.cancel",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/intents/{paymentIntentId}/attempts",
        "commerce",
        "payments.intents.attempts.create",
    )
    .with_idempotent(true),
    // === Payment Method / Record / Attempt / Statistics / Status ===
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/methods",
        "commerce",
        "payments.methods.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/records",
        "commerce",
        "payments.records.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/records/{paymentId}",
        "commerce",
        "payments.records.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/attempts/{paymentAttemptId}",
        "commerce",
        "payments.attempts.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/statistics/summary",
        "commerce",
        "payments.statistics.summary.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/checkout/{paymentId}",
        "commerce",
        "payments.checkout.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/status/{paymentId}",
        "commerce",
        "payments.status.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/payments/status/out_trade_no/{outTradeNo}",
        "commerce",
        "payments.status.outTradeNo.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments",
        "commerce",
        "payments.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments:reconcile",
        "commerce",
        "payments.reconcile",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/payments/{paymentId}/close",
        "commerce",
        "payments.close",
    )
    .with_idempotent(true),
    HttpRoute::public(
        HttpMethod::Post,
        // Deprecated 410 shim 鈥?live PSP webhooks: order POST /orders/payments/webhooks/{providerCode}
        "/app/v3/api/payments/webhooks/{providerCode}",
        "commerce",
        "payments.webhooks.receiveDeprecated",
    ),
    // === Refund ===
    HttpRoute::dual_token(
        HttpMethod::Post,
        "/app/v3/api/refunds",
        "commerce",
        "refunds.create",
    )
    .with_idempotent(true),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/refunds",
        "commerce",
        "refunds.list",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/refunds/{refundId}",
        "commerce",
        "refunds.retrieve",
    ),
];

/// 鏋勯€?payment app-api 鐨?route manifest銆?pub fn app_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdkwork_web_core::RouteAuth;
    use std::collections::BTreeSet;

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
        // 鑷冲皯瑕嗙洊鏍稿績鍐欐搷浣滐細create payment / intent / refund
        assert!(idempotent_post_routes.contains(&"payments.create"));
        assert!(idempotent_post_routes.contains(&"payments.intents.create"));
        assert!(idempotent_post_routes.contains(&"refunds.create"));
    }

    #[test]
    fn manifest_excludes_recharge_routes() {
        let manifest = app_route_manifest();
        for route in manifest.routes() {
            assert!(
                !route.path.contains("/recharges"),
                "payment app-api must not declare recharge routes: {:?} {}",
                route.method,
                route.path,
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

    #[test]
    fn active_manifest_operations_match_app_openapi_authority() {
        let document: serde_json::Value = serde_json::from_str(include_str!(
            "../../../apis/app-api/payment/sdkwork-payment-app-api.openapi.yaml"
        ))
        .expect("payment app OpenAPI must be valid JSON-compatible YAML");
        let openapi_operations = openapi_operations(&document);
        let manifest_operations = app_route_manifest()
            .routes()
            .iter()
            .filter(|route| route.operation_id != "payments.webhooks.receiveDeprecated")
            .map(|route| {
                (
                    method_label(route.method).to_owned(),
                    route.path.to_owned(),
                    route.operation_id.to_owned(),
                )
            })
            .collect::<BTreeSet<_>>();

        assert_eq!(openapi_operations, manifest_operations);
    }

    fn openapi_operations(document: &serde_json::Value) -> BTreeSet<(String, String, String)> {
        let mut operations = BTreeSet::new();
        for (path, path_item) in document["paths"].as_object().expect("paths object") {
            for method in ["get", "post", "put", "patch", "delete"] {
                let Some(operation) = path_item.get(method) else {
                    continue;
                };
                operations.insert((
                    method.to_owned(),
                    path.to_owned(),
                    operation["operationId"]
                        .as_str()
                        .expect("operationId")
                        .to_owned(),
                ));
            }
        }
        operations
    }

    fn method_label(method: HttpMethod) -> &'static str {
        match method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Put => "put",
            HttpMethod::Patch => "patch",
            HttpMethod::Delete => "delete",
        }
    }
}
