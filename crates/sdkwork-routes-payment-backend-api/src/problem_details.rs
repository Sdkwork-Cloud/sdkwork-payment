//! C16 修复：RFC 9457 Problem+json 错误响应统一构建器。
//!
//! 符合 `sdkwork-specs/API_SPEC.md` 第 15.2 节 `ProblemDetail` schema：
//! ```yaml
//! ProblemDetail:
//!   type: object
//!   additionalProperties: true
//!   required: [type, title, status]
//!   properties:
//!     type: { type: string, format: uri-reference }
//!     title: { type: string }
//!     status: { type: integer, minimum: 100, maximum: 599 }
//!     detail: { type: string }
//!     instance: { type: string }
//!     code: { type: string }
//!     traceId: { type: string }
//!     errors: { type: array, items: { $ref: "#/components/schemas/FieldError" } }
//! ```
//!
//! 所有错误响应 `MUST` 使用 `application/problem+json` content-type。
//! 成功响应继续保留 `{code, msg, data}` 业务 envelope（API_SPEC §15.1）。

use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

/// RFC 9457 Problem+json 响应体。`additionalProperties: true` 允许扩展字段。
#[derive(Debug, Serialize)]
pub(crate) struct ProblemDetail {
    /// URI reference identifying the problem type.
    #[serde(rename = "type")]
    pub problem_type: &'static str,
    /// Short, human-readable summary of the problem type.
    pub title: &'static str,
    /// HTTP status code.
    pub status: u16,
    /// Human-readable explanation specific to this occurrence.
    pub detail: String,
    /// SDKWork business error code (e.g. "4010", "4040", "4091").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// W3C trace id when available from request context.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "traceId")]
    pub trace_id: Option<String>,
}

impl ProblemDetail {
    /// 构建 Problem+json 响应。
    ///
    /// `problem_type_uri` 应为稳定的 URI reference（如 `https://sdkwork.dev/problems/forbidden`），
    /// 不随实例变化。`title` 应为状态码的 canonical reason。
    pub(crate) fn build(
        status: axum::http::StatusCode,
        problem_type_uri: &'static str,
        title: &'static str,
        code: impl Into<String>,
        detail: impl Into<String>,
    ) -> Response {
        let payload = Self {
            problem_type: problem_type_uri,
            title,
            status: status.as_u16(),
            detail: detail.into(),
            code: Some(code.into()),
            trace_id: None,
        };
        let mut response = (status, Json(payload)).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );
        response
    }
}

/// 按状态码返回 canonical title，避免硬编码。
fn canonical_title(status: axum::http::StatusCode) -> &'static str {
    status.canonical_reason().unwrap_or("Request Error")
}

/// 按状态码返回稳定的 problem type URI。
fn problem_type_uri(status: axum::http::StatusCode) -> &'static str {
    match status {
        axum::http::StatusCode::UNAUTHORIZED => "https://sdkwork.dev/problems/invalid-credentials",
        axum::http::StatusCode::FORBIDDEN => "https://sdkwork.dev/problems/forbidden",
        axum::http::StatusCode::BAD_REQUEST => "https://sdkwork.dev/problems/bad-request",
        axum::http::StatusCode::NOT_FOUND => "https://sdkwork.dev/problems/not-found",
        axum::http::StatusCode::CONFLICT => "https://sdkwork.dev/problems/conflict",
        axum::http::StatusCode::UNPROCESSABLE_ENTITY => {
            "https://sdkwork.dev/problems/unprocessable-entity"
        }
        axum::http::StatusCode::TOO_MANY_REQUESTS => "https://sdkwork.dev/problems/rate-limit-exceeded",
        axum::http::StatusCode::INTERNAL_SERVER_ERROR => {
            "https://sdkwork.dev/problems/internal-server-error"
        }
        axum::http::StatusCode::SERVICE_UNAVAILABLE => {
            "https://sdkwork.dev/problems/dependency-unavailable"
        }
        _ => "https://sdkwork.dev/problems/request-error",
    }
}

/// C16 修复：统一的 Problem+json 错误响应构建入口。
///
/// 替代旧的 `Json(ApiResult::error(...))` 错误响应。
pub(crate) fn problem_error_response(
    status: axum::http::StatusCode,
    code: impl Into<String>,
    detail: impl Into<String>,
) -> Response {
    ProblemDetail::build(
        status,
        problem_type_uri(status),
        canonical_title(status),
        code,
        detail,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problem_response_sets_problem_json_content_type() {
        let response = problem_error_response(
            axum::http::StatusCode::NOT_FOUND,
            "4040",
            "payment intent was not found",
        );
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default();
        assert_eq!("application/problem+json", content_type);
    }

    #[test]
    fn problem_response_includes_required_fields() {
        let response = problem_error_response(
            axum::http::StatusCode::CONFLICT,
            "4091",
            "replay limit exceeded",
        );
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let bytes = rt
            .block_on(async {
                axum::body::to_bytes(response.into_body(), usize::MAX).await
            })
            .expect("body");
        let payload: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(
            "https://sdkwork.dev/problems/conflict",
            payload["type"].as_str().unwrap()
        );
        assert_eq!("Conflict", payload["title"].as_str().unwrap());
        assert_eq!(409, payload["status"].as_u64().unwrap());
        assert_eq!(
            "replay limit exceeded",
            payload["detail"].as_str().unwrap()
        );
        assert_eq!("4091", payload["code"].as_str().unwrap());
    }
}
