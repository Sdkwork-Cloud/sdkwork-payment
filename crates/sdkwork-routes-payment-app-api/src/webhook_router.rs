//! Deprecated payment webhook surface — PSP callbacks must target order-app-api.

use axum::body::Bytes;
use axum::extract::{Extension, Path};
use axum::http::{header, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use sdkwork_utils_rust::{SdkWorkProblemDetail, SdkWorkResultCode};
use sdkwork_web_core::WebRequestContext;

use crate::api_response::resolve_trace_id;

const MIGRATION_DETAIL: &str = "Payment webhooks moved to POST /app/v3/api/orders/payments/webhooks/{providerCode} on the order gateway.";
const WEBHOOK_SHIM_SUNSET_DATE: &str = "Sun, 31 Dec 2026 23:59:59 GMT";
const WEBHOOK_SUCCESSOR_LINK: &str = "https://docs.sdkwork.ai/migrations/payment-webhooks-to-order";

pub fn payment_webhook_router_deprecated() -> Router {
    Router::new().route(
        "/app/v3/api/payments/webhooks/{providerCode}",
        post(receive_provider_webhook_deprecated),
    )
}

async fn receive_provider_webhook_deprecated(
    request_context: Option<Extension<WebRequestContext>>,
    Path(_provider_code): Path<String>,
    _body: Bytes,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let trace_id = resolve_trace_id(ctx);
    let problem =
        SdkWorkProblemDetail::platform(SdkWorkResultCode::Gone, MIGRATION_DETAIL, trace_id.clone());
    attach_webhook_deprecation_headers(problem_json_response(StatusCode::GONE, problem, trace_id))
}

fn problem_json_response(
    status: StatusCode,
    problem: SdkWorkProblemDetail,
    trace_id: String,
) -> Response {
    let mut response = (status, axum::Json(problem)).into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    if let Ok(value) = HeaderValue::from_str(&trace_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-sdkwork-trace-id"), value);
    }
    response
}

fn attach_webhook_deprecation_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("deprecation"),
        HeaderValue::from_static("true"),
    );
    if let Ok(value) = HeaderValue::from_str(WEBHOOK_SHIM_SUNSET_DATE) {
        headers.insert(HeaderName::from_static("sunset"), value);
    }
    let link_value = format!("<{WEBHOOK_SUCCESSOR_LINK}>; rel=\"successor-version\"");
    if let Ok(value) = HeaderValue::from_str(&link_value) {
        headers.insert(header::LINK, value);
    }
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    use axum::body::Body;
    use axum::http::header;

    #[test]
    fn deprecated_webhook_response_uses_problem_json_content_type() {
        let response = problem_json_response(
            StatusCode::GONE,
            SdkWorkProblemDetail::platform(
                SdkWorkResultCode::Gone,
                MIGRATION_DETAIL,
                "trace-test".to_owned(),
            ),
            "trace-test".to_owned(),
        );
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert!(content_type.contains("application/problem+json"));
    }

    #[test]
    fn deprecated_webhook_response_includes_migration_headers() {
        let response = attach_webhook_deprecation_headers(Response::new(Body::empty()));
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
    }
}
