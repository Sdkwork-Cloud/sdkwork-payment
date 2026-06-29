use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, sqlite::SqliteRow, PgPool, Row, SqlitePool};

use crate::problem_details::problem_error_response;
use crate::subject::backend_runtime_subject_from_extension;

pub type CommerceBackendPaymentIntentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceBackendPaymentIntentStore: Send + Sync {
    fn list_payment_intents<'a>(
        &'a self,
        query: BackendPaymentIntentListQuery,
    ) -> CommerceBackendPaymentIntentFuture<'a, Vec<BackendPaymentIntentView>>;

    fn retrieve_payment_intent<'a>(
        &'a self,
        payment_intent_id: String,
        tenant_scope: TenantScope,
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>>;
}

/// C12 修复：Backend 查询必须携带租户范围，避免跨租户数据泄露。
#[derive(Debug, Clone)]
pub struct TenantScope {
    pub tenant_id: String,
    pub organization_id: Option<String>,
}

#[derive(Clone)]
struct BackendPaymentIntentState {
    store: Arc<dyn CommerceBackendPaymentIntentStore>,
}

/// C12 修复：Backend 内部查询结构，tenant_scope 由 IAM context 注入，
/// 不允许从 URL 反序列化，杜绝跨租户越权读取。
#[derive(Debug, Clone)]
pub struct BackendPaymentIntentListQuery {
    pub tenant_scope: TenantScope,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

/// C12 修复：URL 查询参数仅暴露过滤/分页字段，tenant_scope 强制从 IAM context 解析。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentIntentListQueryParams {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentIntentApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentIntentListResponse {
    content: Vec<BackendPaymentIntentResponse>,
}

#[derive(Clone, Debug)]
pub struct BackendPaymentIntentView {
    pub payment_intent_id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
    pub payment_intent_no: String,
    pub payment_method: String,
    pub provider_code: String,
    pub amount: String,
    pub currency_code: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentIntentResponse {
    payment_intent_id: String,
    tenant_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    organization_id: Option<String>,
    owner_user_id: String,
    order_id: String,
    payment_intent_no: String,
    payment_method: String,
    provider_code: String,
    amount: String,
    currency_code: String,
    status: String,
    created_at: String,
    updated_at: String,
}

#[derive(Clone)]
struct SqliteBackendPaymentIntentStore {
    pool: SqlitePool,
}

#[derive(Clone)]
struct PostgresBackendPaymentIntentStore {
    pool: PgPool,
}

impl SqliteBackendPaymentIntentStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PostgresBackendPaymentIntentStore {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl CommerceBackendPaymentIntentStore for SqliteBackendPaymentIntentStore {
    fn list_payment_intents<'a>(
        &'a self,
        query: BackendPaymentIntentListQuery,
    ) -> CommerceBackendPaymentIntentFuture<'a, Vec<BackendPaymentIntentView>> {
        Box::pin(async move {
            // C12 修复：强制 tenant_id 谓词，organization_id 可空匹配，杜绝跨租户数据泄露。
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE tenant_id = CAST(? AS TEXT)
                  AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
                  AND (? IS NULL OR LOWER(COALESCE(status, '')) = LOWER(CAST(? AS TEXT)))
                ORDER BY created_at DESC, id DESC
                "#,
            )
            .bind(&query.tenant_scope.tenant_id)
            .bind(query.tenant_scope.organization_id.as_deref())
            .bind(query.tenant_scope.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.status.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment intents: {error}"))
            })?;

            Ok(rows.iter().map(map_payment_intent_row_sqlite).collect())
        })
    }

    fn retrieve_payment_intent<'a>(
        &'a self,
        payment_intent_id: String,
        tenant_scope: TenantScope,
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>> {
        Box::pin(async move {
            // C12 修复：retrieve 也必须带 tenant 谓词。
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE id = CAST(? AS TEXT)
                  AND tenant_id = CAST(? AS TEXT)
                  AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
                LIMIT 1
                "#,
            )
            .bind(&payment_intent_id)
            .bind(&tenant_scope.tenant_id)
            .bind(tenant_scope.organization_id.as_deref())
            .bind(tenant_scope.organization_id.as_deref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to retrieve payment intent: {error}"))
            })?;

            Ok(row.as_ref().map(map_payment_intent_row_sqlite))
        })
    }
}

impl CommerceBackendPaymentIntentStore for PostgresBackendPaymentIntentStore {
    fn list_payment_intents<'a>(
        &'a self,
        query: BackendPaymentIntentListQuery,
    ) -> CommerceBackendPaymentIntentFuture<'a, Vec<BackendPaymentIntentView>> {
        Box::pin(async move {
            // C12 修复：强制 tenant_id 谓词。
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3 IS NULL))
                  AND ($4::text IS NULL OR LOWER(COALESCE(status, '')) = LOWER($4::text))
                ORDER BY created_at DESC, id DESC
                "#,
            )
            .bind(&query.tenant_scope.tenant_id)
            .bind(query.tenant_scope.organization_id.as_deref())
            .bind(query.tenant_scope.organization_id.as_deref())
            .bind(query.status.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment intents: {error}"))
            })?;

            Ok(rows.iter().map(map_payment_intent_row_pg).collect())
        })
    }

    fn retrieve_payment_intent<'a>(
        &'a self,
        payment_intent_id: String,
        tenant_scope: TenantScope,
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE id = CAST($1 AS TEXT)
                  AND tenant_id = CAST($2 AS TEXT)
                  AND ((organization_id = CAST($3 AS TEXT)) OR (organization_id IS NULL AND $4 IS NULL))
                LIMIT 1
                "#,
            )
            .bind(&payment_intent_id)
            .bind(&tenant_scope.tenant_id)
            .bind(tenant_scope.organization_id.as_deref())
            .bind(tenant_scope.organization_id.as_deref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to retrieve payment intent: {error}"))
            })?;

            Ok(row.as_ref().map(map_payment_intent_row_pg))
        })
    }
}

impl<T: Serialize> BackendPaymentIntentApiResult<T> {
    fn success(data: T) -> Self {
        Self {
            code: "0".to_owned(),
            msg: "success".to_owned(),
            data: Some(data),
        }
    }
}

pub fn backend_payment_intent_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_backend_payment_intent_router(Arc::new(SqliteBackendPaymentIntentStore::new(pool)))
}

pub fn backend_payment_intent_router_with_postgres_pool(pool: PgPool) -> Router {
    build_backend_payment_intent_router(Arc::new(PostgresBackendPaymentIntentStore::new(pool)))
}

pub fn build_backend_payment_intent_router(
    store: Arc<dyn CommerceBackendPaymentIntentStore>,
) -> Router {
    Router::new()
            .route(
                "/backend/v3/api/payments/intents",
                get(list_payment_intents),
            )
            .route(
                "/backend/v3/api/payments/intents/{paymentIntentId}",
                get(retrieve_payment_intent),
            )
            .with_state(BackendPaymentIntentState { store })
}

async fn list_payment_intents(
    State(state): State<BackendPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Query(params): Query<BackendPaymentIntentListQueryParams>,
) -> Response {
    // C12 修复：tenant_scope 必须从 IAM context 解析，绝不接受 URL 传入。
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => {
            return problem_error_response(StatusCode::UNAUTHORIZED, "4010", message);
        }
    };

    let page = params.page.unwrap_or(1).max(1);
    let page_size = params.page_size.unwrap_or(20).clamp(1, 200);
    let query = BackendPaymentIntentListQuery {
        tenant_scope: TenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        status: params.status,
        page: Some(page),
        page_size: Some(page_size),
    };

    match state.store.list_payment_intents(query).await {
        Ok(items) => {
            let start = ((page - 1) * page_size) as usize;
            let content = items
                .into_iter()
                .skip(start)
                .take(page_size as usize)
                .map(map_payment_intent)
                .collect::<Vec<_>>();
            Json(BackendPaymentIntentApiResult::success(
                BackendPaymentIntentListResponse { content },
            ))
            .into_response()
        }
        Err(error) => backend_payment_intent_error_response(
            "payment intent management list is unavailable",
            error,
        ),
    }
}

async fn retrieve_payment_intent(
    State(state): State<BackendPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_intent_id): Path<String>,
) -> Response {
    // C12 修复：retrieve 必须携带 tenant_scope，防止跨租户读取任意 payment intent。
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => {
            return problem_error_response(StatusCode::UNAUTHORIZED, "4010", message);
        }
    };

    let tenant_scope = TenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };

    match state
        .store
        .retrieve_payment_intent(payment_intent_id, tenant_scope)
        .await
    {
        Ok(Some(intent)) => Json(BackendPaymentIntentApiResult::success(map_payment_intent(
            intent,
        )))
        .into_response(),
        Ok(None) => problem_error_response(
            StatusCode::NOT_FOUND,
            "4040",
            "payment intent was not found",
        ),
        Err(error) => backend_payment_intent_error_response(
            "payment intent management read model is unavailable",
            error,
        ),
    }
}

fn backend_payment_intent_error_response(context: &str, error: CommerceServiceError) -> Response {
    // C16 修复：500 错误使用 Problem+json，detail 不泄露内部堆栈。
    problem_error_response(
        StatusCode::INTERNAL_SERVER_ERROR,
        "5000",
        format!("{context}: {}", error.message()),
    )
}

fn map_payment_intent_row_sqlite(row: &SqliteRow) -> BackendPaymentIntentView {
    BackendPaymentIntentView {
        payment_intent_id: sqlite_string(row, "id"),
        tenant_id: sqlite_string(row, "tenant_id"),
        organization_id: sqlite_optional_string(row, "organization_id"),
        owner_user_id: sqlite_string(row, "owner_user_id"),
        order_id: sqlite_string(row, "order_id"),
        payment_intent_no: sqlite_string(row, "payment_intent_no"),
        payment_method: sqlite_string(row, "payment_method"),
        provider_code: sqlite_string(row, "provider_code"),
        amount: sqlite_string(row, "amount"),
        currency_code: sqlite_string(row, "currency_code"),
        status: sqlite_string(row, "status"),
        created_at: sqlite_string(row, "created_at"),
        updated_at: sqlite_string(row, "updated_at"),
    }
}

fn map_payment_intent_row_pg(row: &PgRow) -> BackendPaymentIntentView {
    BackendPaymentIntentView {
        payment_intent_id: pg_string(row, "id"),
        tenant_id: pg_string(row, "tenant_id"),
        organization_id: pg_optional_string(row, "organization_id"),
        owner_user_id: pg_string(row, "owner_user_id"),
        order_id: pg_string(row, "order_id"),
        payment_intent_no: pg_string(row, "payment_intent_no"),
        payment_method: pg_string(row, "payment_method"),
        provider_code: pg_string(row, "provider_code"),
        amount: pg_string(row, "amount"),
        currency_code: pg_string(row, "currency_code"),
        status: pg_string(row, "status"),
        created_at: pg_string(row, "created_at"),
        updated_at: pg_string(row, "updated_at"),
    }
}

fn map_payment_intent(value: BackendPaymentIntentView) -> BackendPaymentIntentResponse {
    BackendPaymentIntentResponse {
        payment_intent_id: value.payment_intent_id,
        tenant_id: value.tenant_id,
        organization_id: value.organization_id,
        owner_user_id: value.owner_user_id,
        order_id: value.order_id,
        payment_intent_no: value.payment_intent_no,
        payment_method: value.payment_method,
        provider_code: value.provider_code,
        amount: value.amount,
        currency_code: value.currency_code,
        status: value.status,
        created_at: value.created_at,
        updated_at: value.updated_at,
    }
}

fn sqlite_optional_string(row: &SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn sqlite_string(row: &SqliteRow, column: &str) -> String {
    sqlite_optional_string(row, column).unwrap_or_default()
}

fn pg_optional_string(row: &PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn pg_string(row: &PgRow, column: &str) -> String {
    pg_optional_string(row, column).unwrap_or_default()
}
