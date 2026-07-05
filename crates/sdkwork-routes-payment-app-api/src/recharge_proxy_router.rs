//! Deprecated `/app/v3/api/recharges/*` surface — forwards to sdkwork-order (P2).
//!
//! **Deprecation**: 本路由为兼容代理，将在一个发布周期后移除。客户端 `MUST` 直接消费
//! `@sdkwork/order-app-sdk` 而非本代理。所有响应附带 `Deprecation: true`、
//! `Sunset` 与 `Link: rel="successor-version"` 头部（RFC 7234 / RFC 8594）。
//!
//! **Spec compliance**: 上游 `sdkwork-order` 已对齐 `API_SPEC.md` §4.5/§15，本代理
//! 透传 `SdkWorkApiResponse` 成功信封与 `application/problem+json` 错误信封；若上游
//! 返回非合规响应（例如裸 `{code, msg, data}` 旧信封或非 JSON body），代理 `MUST`
//! 返回 502 Bad Gateway Problem+json（`API_SPEC.md` §15.3 `BadGateway`）。

use std::collections::BTreeMap;
use std::sync::OnceLock;
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::{Extension, Path, Query};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_utils_rust::{SdkWorkApiResponse, SdkWorkProblemDetail, SdkWorkResultCode};
use sdkwork_web_core::WebRequestContext;
use serde_json::Value;

use crate::api_response::{bad_gateway, resolve_trace_id};

/// HTTP 客户端上游请求超时（避免上游 hang 导致连接耗尽/OOM）。
const UPSTREAM_REQUEST_TIMEOUT: Duration = Duration::from_secs(15);

/// 代理 deprecation sunset 日期（RFC 8594）。客户端应在此时点前完成迁移。
const RECHARGE_PROXY_SUNSET_DATE: &str = "Sun, 31 Dec 2026 23:59:59 GMT";

/// 上游 successor 文档 URL（`Link: rel="successor-version"` 指向迁移目标）。
const RECHARGE_PROXY_SUCCESSOR_LINK: &str =
    "https://docs.sdkwork.ai/migrations/recharges-to-orders";

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(UPSTREAM_REQUEST_TIMEOUT)
            .connect_timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client builder must not fail with valid options")
    })
}

fn order_app_api_origin() -> String {
    std::env::var("SDKWORK_ORDER_APP_API_ORIGIN")
        .unwrap_or_else(|_| "http://127.0.0.1:18093".to_string())
        .trim()
        .trim_end_matches('/')
        .to_string()
}

pub fn app_recharge_proxy_router() -> Router {
    Router::new()
        .route(
            "/app/v3/api/recharges/packages",
            get(proxy_recharge_packages),
        )
        .route(
            "/app/v3/api/recharges/settings",
            get(proxy_recharge_settings),
        )
        .route(
            "/app/v3/api/recharges/orders",
            get(proxy_recharge_orders_list).post(proxy_recharge_orders_create),
        )
        .route(
            "/app/v3/api/recharges/orders/{orderId}",
            get(proxy_recharge_order_retrieve),
        )
        .route(
            "/app/v3/api/recharges/orders/{orderId}/cancel",
            post(proxy_recharge_order_cancel),
        )
}

async fn proxy_recharge_packages(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
) -> Response {
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::GET,
        "/app/v3/api/recharges/packages",
        None,
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_settings(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
) -> Response {
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::GET,
        "/app/v3/api/recharges/settings",
        None,
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_orders_list(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Query(params): Query<BTreeMap<String, String>>,
) -> Response {
    let query = build_query_string(&params);
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::GET,
        "/app/v3/api/recharges/orders",
        query.as_deref(),
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_orders_create(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let payload = serde_json::to_vec(&body).unwrap_or_default();
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::POST,
        "/app/v3/api/recharges/orders",
        None,
        headers,
        Some(Bytes::from(payload)),
    )
    .await
}

async fn proxy_recharge_order_retrieve(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Response {
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::GET,
        &format!("/app/v3/api/recharges/orders/{order_id}"),
        None,
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_order_cancel(
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<Value>>,
) -> Response {
    let payload = body
        .map(|Json(value)| serde_json::to_vec(&value).unwrap_or_default())
        .unwrap_or_default();
    forward(
        request_context.as_ref().map(|Extension(value)| value),
        Method::POST,
        &format!("/app/v3/api/recharges/orders/{order_id}/cancel"),
        None,
        headers,
        Some(Bytes::from(payload)),
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn forward(
    context: Option<&WebRequestContext>,
    method: Method,
    path: &str,
    query: Option<&str>,
    headers: HeaderMap,
    body: Option<Bytes>,
) -> Response {
    let mut url = format!("{}{}", order_app_api_origin(), path);
    if let Some(query_string) = query.filter(|value| !value.is_empty()) {
        url.push('?');
        url.push_str(query_string);
    }

    let trace_id = resolve_trace_id(context);

    let mut builder = http_client().request(method, &url);
    builder = builder.header("x-sdkwork-recharge-proxy", "payment");
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        builder = builder.header("x-sdkwork-trace-id", value);
    }
    for (name, value) in headers.iter() {
        let name_str = name.as_str();
        if matches!(
            name_str,
            "host" | "connection" | "content-length" | "transfer-encoding"
        ) {
            continue;
        }
        if let Ok(value_str) = value.to_str() {
            builder = builder.header(name_str, value_str);
        }
    }
    if let Some(body) = body {
        builder = builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(body);
    }

    let upstream_result = builder.send().await;
    let upstream = match upstream_result {
        Ok(response) => response,
        Err(error) => {
            return bad_gateway(
                context,
                format!("order recharge proxy unavailable: {error}"),
            )
        }
    };

    let mapped = map_upstream_response(context, upstream).await;
    attach_deprecation_headers(mapped, &trace_id)
}

async fn map_upstream_response(
    context: Option<&WebRequestContext>,
    upstream: reqwest::Response,
) -> Response {
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let headers = upstream.headers().clone();
    let body = upstream.bytes().await.unwrap_or_default();

    // 上游响应合规性检查：成功必须是 SdkWorkApiResponse；错误必须是 problem+json。
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("")
        .to_lowercase();

    if !is_compliant_response(status, &content_type, &body) {
        return bad_gateway(
            context,
            "order recharge upstream returned non-compliant response envelope",
        );
    }

    let mut response = Response::builder().status(status);
    for (name, value) in headers.iter() {
        if name == header::TRANSFER_ENCODING || name == header::CONTENT_LENGTH {
            continue;
        }
        response = response.header(name, value);
    }
    response
        .header("x-sdkwork-recharge-proxy", "payment")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}

/// 校验上游响应是否符合 `API_SPEC.md` §4.5/§15。
///
/// - 2xx：`Content-Type: application/json`，body 必须解析为 `SdkWorkApiResponse`
///   且 `code == 0` 与 `data` 字段同时存在。
/// - 4xx/5xx：`Content-Type: application/problem+json`。
/// - 其它（非 JSON、`code != 0` 成功体、非 problem+json 错误体）一律视为非合规。
fn is_compliant_response(status: StatusCode, content_type: &str, body: &[u8]) -> bool {
    if status.is_success() {
        if !content_type.starts_with("application/json") {
            return false;
        }
        let Ok(parsed) = serde_json::from_slice::<Value>(body) else {
            return false;
        };
        let Some(code) = parsed.get("code") else {
            return false;
        };
        let code_matches = match code {
            Value::Number(number) => number.as_i64() == Some(0),
            Value::String(value) => value.trim() == "0",
            _ => false,
        };
        if !code_matches {
            return false;
        }
        return parsed.get("data").is_some();
    }

    if status.is_client_error() || status.is_server_error() {
        return content_type.starts_with("application/problem+json");
    }

    // 3xx、1xx 等非典型状态码视为非合规。
    false
}

/// 给响应注入 RFC 7234 `Deprecation`、RFC 8594 `Sunset` 与 `Link: rel="successor-version"` 头部。
fn attach_deprecation_headers(mut response: Response, trace_id: &str) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("deprecation"),
        HeaderValue::from_static("true"),
    );
    if let Ok(value) = HeaderValue::from_str(RECHARGE_PROXY_SUNSET_DATE) {
        headers.insert(HeaderName::from_static("sunset"), value);
    }
    let link_value = format!(
        "<{}>; rel=\"successor-version\"",
        RECHARGE_PROXY_SUCCESSOR_LINK
    );
    if let Ok(value) = HeaderValue::from_str(&link_value) {
        headers.insert(header::LINK, value);
    }
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        headers.insert(
            HeaderName::from_static("x-sdkwork-trace-id"),
            value,
        );
    }
    response
}

fn build_query_string(params: &BTreeMap<String, String>) -> Option<String> {
    if params.is_empty() {
        return None;
    }
    Some(
        params
            .iter()
            .map(|(key, value)| format!("{}={}", urlencoding(key), urlencoding(value)))
            .collect::<Vec<_>>()
            .join("&"),
    )
}

fn urlencoding(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.' | '~') {
                character.to_string()
            } else {
                format!("%{:02X}", character as u32)
            }
        })
        .collect()
}

/// 构造用于测试的合规成功响应示例（仅供 `tests` 模块使用）。
#[cfg(test)]
fn compliant_success_body(item: &Value) -> Vec<u8> {
    let envelope = SdkWorkApiResponse::success(item.to_owned(), "test-trace".to_string());
    serde_json::to_vec(&envelope).unwrap_or_default()
}

/// 构造用于测试的合规 problem+json 错误响应示例（仅供 `tests` 模块使用）。
#[cfg(test)]
fn compliant_problem_body(result_code: SdkWorkResultCode, detail: &str) -> Vec<u8> {
    let problem = SdkWorkProblemDetail::platform(result_code, detail.to_string(), "test-trace");
    serde_json::to_vec(&problem).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn is_compliant_response_accepts_sdkwork_success_envelope() {
        let item = serde_json::json!({"packageId": "P-1"});
        let body = compliant_success_body(&item);
        assert!(is_compliant_response(
            StatusCode::OK,
            "application/json; charset=utf-8",
            &body,
        ));
    }

    #[test]
    fn is_compliant_response_rejects_legacy_success_envelope() {
        let body = br#"{"code":200,"msg":"ok","data":{"packageId":"P-1"}}"#;
        assert!(!is_compliant_response(
            StatusCode::OK,
            "application/json",
            body,
        ));
    }

    #[test]
    fn is_compliant_response_rejects_success_without_data_field() {
        let body = br#"{"code":0,"traceId":"t-1"}"#;
        assert!(!is_compliant_response(
            StatusCode::OK,
            "application/json",
            body,
        ));
    }

    #[test]
    fn is_compliant_response_accepts_problem_json_error() {
        let body = compliant_problem_body(SdkWorkResultCode::NotFound, "package not found");
        assert!(is_compliant_response(
            StatusCode::NOT_FOUND,
            "application/problem+json; charset=utf-8",
            &body,
        ));
    }

    #[test]
    fn is_compliant_response_rejects_legacy_error_envelope() {
        let body = br#"{"code":404,"message":"not found"}"#;
        assert!(!is_compliant_response(
            StatusCode::NOT_FOUND,
            "application/json",
            body,
        ));
    }

    #[tokio::test]
    async fn attach_deprecation_headers_sets_all_required_headers() {
        let response = Response::new(Body::empty());
        let trace_id = "trace-deprecation-1";
        let response = attach_deprecation_headers(response, trace_id);
        let headers = response.headers();
        assert_eq!(
            headers.get("deprecation").and_then(|v| v.to_str().ok()),
            Some("true"),
        );
        assert!(headers
            .get("sunset")
            .and_then(|v| v.to_str().ok())
            .is_some());
        let link = headers
            .get(header::LINK)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(link.contains("rel=\"successor-version\""));
        assert_eq!(
            headers
                .get("x-sdkwork-trace-id")
                .and_then(|v| v.to_str().ok()),
            Some(trace_id),
        );
    }

    #[tokio::test]
    async fn attach_deprecation_headers_preserves_existing_body() {
        let body_bytes = b"{\"code\":0,\"data\":{\"ok\":true}}".to_vec();
        let response = Response::builder()
            .status(StatusCode::OK)
            .body(Body::from(body_bytes.clone()))
            .unwrap();
        let response = attach_deprecation_headers(response, "trace-1");
        let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
        assert_eq!(bytes.as_ref(), body_bytes.as_slice());
    }
}
