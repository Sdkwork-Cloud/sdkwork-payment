use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_utils_rust::{
    SdkWorkApiResponse, SdkWorkProblemDetail, SdkWorkResourceData, SdkWorkResultCode,
};
use sdkwork_web_core::WebRequestContext;

pub fn resolve_trace_id(context: Option<&WebRequestContext>) -> String {
    context
        .and_then(|ctx| ctx.trace_id.clone())
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| sdkwork_utils_rust::uuid())
}

pub fn success_item<T: serde::Serialize>(
    context: Option<&WebRequestContext>,
    item: T,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let envelope = SdkWorkApiResponse::success(SdkWorkResourceData { item }, trace_id.clone());
    attach_trace_header((StatusCode::OK, Json(envelope)).into_response(), &trace_id)
}

pub fn map_service_error(
    context: Option<&WebRequestContext>,
    error: CommerceServiceError,
) -> Response {
    let trace_id = resolve_trace_id(context);
    let (status, result_code, detail) = match error.code() {
        "validation" => (
            StatusCode::BAD_REQUEST,
            SdkWorkResultCode::ValidationError,
            error.message().to_string(),
        ),
        "not-found" => (
            StatusCode::NOT_FOUND,
            SdkWorkResultCode::NotFound,
            error.message().to_string(),
        ),
        "conflict" => (
            StatusCode::CONFLICT,
            SdkWorkResultCode::Conflict,
            error.message().to_string(),
        ),
        "unauthorized" => (
            StatusCode::UNAUTHORIZED,
            SdkWorkResultCode::AuthenticationRequired,
            error.message().to_string(),
        ),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            SdkWorkResultCode::InternalError,
            error.message().to_string(),
        ),
    };
    let problem = SdkWorkProblemDetail::platform(result_code, detail, trace_id.clone());
    attach_trace_header((status, Json(problem)).into_response(), &trace_id)
}

pub fn unauthorized(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::AuthenticationRequired,
        detail,
        trace_id.clone(),
    );
    attach_trace_header((StatusCode::UNAUTHORIZED, Json(problem)).into_response(), &trace_id)
}

pub fn forbidden(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::PermissionRequired,
        detail,
        trace_id.clone(),
    );
    attach_trace_header((StatusCode::FORBIDDEN, Json(problem)).into_response(), &trace_id)
}

pub fn validation(context: Option<&WebRequestContext>, detail: impl Into<String>) -> Response {
    let trace_id = resolve_trace_id(context);
    let problem = SdkWorkProblemDetail::platform(
        SdkWorkResultCode::ValidationError,
        detail,
        trace_id.clone(),
    );
    attach_trace_header((StatusCode::BAD_REQUEST, Json(problem)).into_response(), &trace_id)
}

fn attach_trace_header(response: Response, trace_id: &str) -> Response {
    let mut response = response;
    if let Ok(value) = HeaderValue::from_str(trace_id) {
        response.headers_mut().insert(
            HeaderName::from_static("x-sdkwork-trace-id"),
            value,
        );
    }
    response
}
