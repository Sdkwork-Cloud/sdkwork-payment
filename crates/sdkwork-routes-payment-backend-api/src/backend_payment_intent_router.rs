use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use sdkwork_commerce_contract_service::CommerceServiceError;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, sqlite::SqliteRow, PgPool, Row, SqlitePool};

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
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>>;
}

#[derive(Clone)]
struct BackendPaymentIntentState {
    store: Arc<dyn CommerceBackendPaymentIntentStore>,
}

#[derive(Debug, Deserialize)]
pub struct BackendPaymentIntentListQuery {
    status: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize", alias = "page_size")]
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
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE (? IS NULL OR LOWER(COALESCE(status, '')) = LOWER(CAST(? AS TEXT)))
                ORDER BY created_at DESC, id DESC
                "#,
            )
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
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE id = CAST(? AS TEXT)
                LIMIT 1
                "#,
            )
            .bind(&payment_intent_id)
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
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE ($1::text IS NULL OR LOWER(COALESCE(status, '')) = LOWER($1::text))
                ORDER BY created_at DESC, id DESC
                "#,
            )
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
    ) -> CommerceBackendPaymentIntentFuture<'a, Option<BackendPaymentIntentView>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                       payment_method, provider_code, CAST(amount AS TEXT) AS amount,
                       currency_code, status, created_at, updated_at
                FROM commerce_payment_intent
                WHERE id = CAST($1 AS TEXT)
                LIMIT 1
                "#,
            )
            .bind(&payment_intent_id)
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

    fn error(code: &str, msg: impl Into<String>) -> Self {
        Self {
            code: code.to_owned(),
            msg: msg.into(),
            data: None,
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
    Query(query): Query<BackendPaymentIntentListQuery>,
) -> Response {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 200);
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
    Path(payment_intent_id): Path<String>,
) -> Response {
    match state.store.retrieve_payment_intent(payment_intent_id).await {
        Ok(Some(intent)) => Json(BackendPaymentIntentApiResult::success(map_payment_intent(
            intent,
        )))
        .into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(BackendPaymentIntentApiResult::<()>::error(
                "4040",
                "payment intent was not found",
            )),
        )
            .into_response(),
        Err(error) => backend_payment_intent_error_response(
            "payment intent management read model is unavailable",
            error,
        ),
    }
}

fn backend_payment_intent_error_response(context: &str, error: CommerceServiceError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(BackendPaymentIntentApiResult::<()>::error(
            "5000",
            format!("{context}: {}", error.message()),
        )),
    )
        .into_response()
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
