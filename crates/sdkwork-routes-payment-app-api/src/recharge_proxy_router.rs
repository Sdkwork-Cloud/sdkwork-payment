//! Deprecated `/app/v3/api/recharges/*` surface — forwards to sdkwork-order (P2).

use std::collections::BTreeMap;
use std::sync::OnceLock;

use axum::body::{Body, Bytes};
use axum::extract::{Path, Query};
use axum::http::{header, HeaderMap, Method, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::Value;

use crate::problem_details::problem_error_response;

static HTTP: OnceLock<reqwest::Client> = OnceLock::new();

fn http_client() -> &'static reqwest::Client {
    HTTP.get_or_init(reqwest::Client::new)
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

async fn proxy_recharge_packages(headers: HeaderMap) -> Response {
    forward(Method::GET, "/app/v3/api/recharges/packages", None, headers, None).await
}

async fn proxy_recharge_settings(headers: HeaderMap) -> Response {
    forward(Method::GET, "/app/v3/api/recharges/settings", None, headers, None).await
}

async fn proxy_recharge_orders_list(
    headers: HeaderMap,
    Query(params): Query<BTreeMap<String, String>>,
) -> Response {
    let query = build_query_string(&params);
    forward(
        Method::GET,
        "/app/v3/api/recharges/orders",
        query.as_deref(),
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_orders_create(headers: HeaderMap, Json(body): Json<Value>) -> Response {
    let payload = serde_json::to_vec(&body).unwrap_or_default();
    forward(
        Method::POST,
        "/app/v3/api/recharges/orders",
        None,
        headers,
        Some(Bytes::from(payload)),
    )
    .await
}

async fn proxy_recharge_order_retrieve(
    headers: HeaderMap,
    Path(order_id): Path<String>,
) -> Response {
    forward(
        Method::GET,
        &format!("/app/v3/api/recharges/orders/{order_id}"),
        None,
        headers,
        None,
    )
    .await
}

async fn proxy_recharge_order_cancel(
    headers: HeaderMap,
    Path(order_id): Path<String>,
    body: Option<Json<Value>>,
) -> Response {
    let payload = body
        .map(|Json(value)| serde_json::to_vec(&value).unwrap_or_default())
        .unwrap_or_default();
    forward(
        Method::POST,
        &format!("/app/v3/api/recharges/orders/{order_id}/cancel"),
        None,
        headers,
        Some(Bytes::from(payload)),
    )
    .await
}

async fn forward(
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

    let mut builder = http_client().request(method, &url);
    builder = builder.header("x-sdkwork-recharge-proxy", "payment");
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

    match builder.send().await {
        Ok(upstream) => map_upstream_response(upstream).await,
        Err(error) => problem_error_response(
            StatusCode::BAD_GATEWAY,
            "5020",
            format!("order recharge proxy unavailable: {error}"),
        ),
    }
}

async fn map_upstream_response(upstream: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(upstream.status().as_u16())
        .unwrap_or(StatusCode::BAD_GATEWAY);
    let headers = upstream.headers().clone();
    let body = upstream
        .bytes()
        .await
        .unwrap_or_default();

    let mut response = Response::builder().status(status);
    for (name, value) in headers.iter() {
        if name == header::TRANSFER_ENCODING {
            continue;
        }
        response = response.header(name, value);
    }
    response
        .header("x-sdkwork-recharge-proxy", "payment")
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
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
