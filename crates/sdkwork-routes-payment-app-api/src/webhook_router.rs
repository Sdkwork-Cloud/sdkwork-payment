//! Deprecated payment webhook surface — PSP callbacks must target order-app-api.

use axum::body::Bytes;
use axum::extract::{Extension, Path};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use sdkwork_utils_rust::{SdkWorkProblemDetail, SdkWorkResultCode};
use sdkwork_web_core::WebRequestContext;

use crate::api_response::resolve_trace_id;

const MIGRATION_DETAIL: &str = "Payment webhooks moved to POST /app/v3/api/orders/payments/webhooks/{providerCode} on the order gateway.";

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
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::Gone,
        MIGRATION_DETAIL,
        trace_id,
    );
    (StatusCode::GONE, axum::Json(problem)).into_response()
}
