use axum::http::{header::AUTHORIZATION, HeaderMap};
use axum::Extension;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_iam_web_adapter::iam_app_context_from_web_principal;
use sdkwork_web_core::{DefaultWebRequestContextResolver, WebRequestContextResolver};

#[derive(Debug, Clone)]
pub(crate) struct AppRuntimeSubject {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub user_id: String,
}

pub(crate) fn app_runtime_subject_from_extension(
    context: Option<Extension<IamAppContext>>,
) -> Result<AppRuntimeSubject, String> {
    let Some(Extension(context)) = context else {
        return Err("authenticated runtime context is required".to_owned());
    };
    app_runtime_subject_from_iam(&context)
}

pub(crate) fn app_runtime_subject_from_iam(
    context: &IamAppContext,
) -> Result<AppRuntimeSubject, String> {
    let tenant_id = required_context_text(&context.tenant_id, "tenant_id")?;
    let user_id = required_context_text(&context.user_id, "user_id")?;
    let organization_id = context
        .organization_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    Ok(AppRuntimeSubject {
        tenant_id,
        organization_id,
        user_id,
    })
}

/// Manifest-public routes skip framework auth; optional dual-token headers still scope reads.
pub(crate) async fn optional_app_runtime_subject_from_headers(
    runtime_context: Option<Extension<IamAppContext>>,
    headers: &HeaderMap,
) -> Option<AppRuntimeSubject> {
    if let Ok(subject) = app_runtime_subject_from_extension(runtime_context) {
        return Some(subject);
    }
    let auth_header = headers.get(AUTHORIZATION)?.to_str().ok()?;
    let auth_token = auth_header
        .strip_prefix("Bearer ")
        .or_else(|| auth_header.strip_prefix("bearer "))
        .unwrap_or(auth_header)
        .trim();
    let access_token = headers.get("Access-Token")?.to_str().ok()?.trim();
    if auth_token.is_empty() || access_token.is_empty() {
        return None;
    }
    let resolver = DefaultWebRequestContextResolver::default();
    let principal = resolver
        .resolve_dual_token(auth_token, access_token)
        .await
        .ok()?;
    app_runtime_subject_from_iam(&iam_app_context_from_web_principal(&principal)).ok()
}

fn required_context_text(value: &str, field_name: &'static str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!(
            "authenticated runtime context {field_name} is required"
        ));
    }
    Ok(value.to_owned())
}
