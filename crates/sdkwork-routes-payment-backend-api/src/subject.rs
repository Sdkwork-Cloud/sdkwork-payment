use axum::http::{header::AUTHORIZATION, HeaderMap};
use axum::Extension;
use sdkwork_iam_context_service::{IamAppContext, LoginScope};
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

/// C11/C12 修复：Backend handler 公开入口，强制执行以下 IAM 边界：
/// 1. 必须存在已认证的 IAM 上下文（dual-token 已由 web framework 解析）。
/// 2. `login_scope` 必须为 `Organization`，拒绝个人会话（`Tenant`），
///    符合 SECURITY_SPEC.md 第 57 行："Backend API requests MUST reject personal sessions"。
/// 3. `can_access_backend_api()` 必须为 true，由 IAM user_surface 控制后台访问开关。
/// 4. 必须携带有效的 `tenant_id` 与 `organization_id`，用于 SQL 谓词注入，
///    杜绝跨租户数据泄露（C12）。
pub(crate) fn backend_runtime_subject_from_extension(
    context: Option<Extension<IamAppContext>>,
) -> Result<AppRuntimeSubject, String> {
    let Some(Extension(context)) = context else {
        return Err("authenticated runtime context is required".to_owned());
    };

    if context.login_scope != LoginScope::Organization {
        return Err(
            "backend api requires an organization session; personal tenant sessions are rejected"
                .to_owned(),
        );
    }

    if !context.can_access_backend_api() {
        return Err("principal is not permitted to access backend api surface".to_owned());
    }

    let subject = app_runtime_subject_from_iam(&context)?;
    if subject.organization_id.is_none() {
        return Err(
            "backend api requires a non-empty organization_id for tenant scoping".to_owned(),
        );
    }
    Ok(subject)
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
