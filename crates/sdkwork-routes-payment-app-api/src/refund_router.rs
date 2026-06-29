use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_service::{
    CreateOwnerRefundCommand, RefundDetailQuery, RefundListQuery, RefundView,
};
use sdkwork_payment_repository_sqlx::{
    PostgresCommerceRefundStore, SqliteCommerceRefundStore,
};
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::command_headers::validate_app_write_payload;
use crate::problem_details::problem_error_response;
use crate::subject::app_runtime_subject_from_extension;

pub type CommerceRefundFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceRefundStore: Send + Sync {
    fn create_owner_refund<'a>(
        &'a self,
        command: CreateOwnerRefundCommand,
    ) -> CommerceRefundFuture<'a, RefundView>;

    fn list_owner_refunds<'a>(
        &'a self,
        query: RefundListQuery,
    ) -> CommerceRefundFuture<'a, Vec<RefundView>>;

    fn retrieve_owner_refund<'a>(
        &'a self,
        query: RefundDetailQuery,
    ) -> CommerceRefundFuture<'a, Option<RefundView>>;
}

#[derive(Clone)]
struct AppRefundState {
    store: Arc<dyn CommerceRefundStore>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateRefundBody {
    order_id: String,
    #[serde(rename = "paymentAttemptId", alias = "payment_attempt_id")]
    payment_attempt_id: Option<String>,
    amount: Option<String>,
    #[serde(rename = "reasonCode", alias = "reason_code")]
    reason_code: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefundListParams {
    status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppRefundApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RefundResponse {
    refund_id: String,
    refund_no: String,
    order_id: String,
    payment_attempt_id: String,
    amount: String,
    currency_code: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<String>,
}

impl CommerceRefundStore for SqliteCommerceRefundStore {
    fn create_owner_refund<'a>(
        &'a self,
        command: CreateOwnerRefundCommand,
    ) -> CommerceRefundFuture<'a, RefundView> {
        Box::pin(async move { self.create_owner_refund(command).await })
    }

    fn list_owner_refunds<'a>(
        &'a self,
        query: RefundListQuery,
    ) -> CommerceRefundFuture<'a, Vec<RefundView>> {
        Box::pin(async move { self.list_owner_refunds(query).await })
    }

    fn retrieve_owner_refund<'a>(
        &'a self,
        query: RefundDetailQuery,
    ) -> CommerceRefundFuture<'a, Option<RefundView>> {
        Box::pin(async move { self.retrieve_owner_refund(query).await })
    }
}

impl CommerceRefundStore for PostgresCommerceRefundStore {
    fn create_owner_refund<'a>(
        &'a self,
        command: CreateOwnerRefundCommand,
    ) -> CommerceRefundFuture<'a, RefundView> {
        Box::pin(async move { self.create_owner_refund(command).await })
    }

    fn list_owner_refunds<'a>(
        &'a self,
        query: RefundListQuery,
    ) -> CommerceRefundFuture<'a, Vec<RefundView>> {
        Box::pin(async move { self.list_owner_refunds(query).await })
    }

    fn retrieve_owner_refund<'a>(
        &'a self,
        query: RefundDetailQuery,
    ) -> CommerceRefundFuture<'a, Option<RefundView>> {
        Box::pin(async move { self.retrieve_owner_refund(query).await })
    }
}

impl<T: Serialize> AppRefundApiResult<T> {
    fn success(data: T) -> Self {
        Self {
            code: "0".to_owned(),
            msg: "success".to_owned(),
            data: Some(data),
        }
    }
}

pub fn app_refund_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_app_refund_router(Arc::new(SqliteCommerceRefundStore::new(pool)))
}

pub fn app_refund_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_refund_router(Arc::new(PostgresCommerceRefundStore::new(pool)))
}

pub fn build_app_refund_router(store: Arc<dyn CommerceRefundStore>) -> Router {
    Router::new()
            .route("/app/v3/api/refunds", post(create_refund).get(list_refunds))
            .route("/app/v3/api/refunds/{refundId}", get(retrieve_refund))
            .with_state(AppRefundState { store })
}

async fn create_refund(
    State(state): State<AppRefundState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<CreateRefundBody>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let write_headers =
        match validate_app_write_payload(&headers, "refunds.create", &body, |idempotency_key| {
            format!("refund-{}-{}", subject.user_id, idempotency_key)
        }) {
            Ok(value) => value,
            Err(response) => return response,
        };
    let command = match CreateOwnerRefundCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &body.order_id,
        body.payment_attempt_id.as_deref(),
        body.amount.as_deref(),
        body.reason_code.as_deref(),
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.create_owner_refund(command).await {
        Ok(refund) => Json(AppRefundApiResult::success(map_refund(refund))).into_response(),
        Err(error) => refund_system_response("refund create failed", error),
    }
}

async fn list_refunds(
    State(state): State<AppRefundState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Query(params): Query<RefundListParams>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match RefundListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        params.status.as_deref(),
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.list_owner_refunds(query).await {
        Ok(refunds) => Json(AppRefundApiResult::success(
            refunds.into_iter().map(map_refund).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => refund_system_response("refund list is unavailable", error),
    }
}

async fn retrieve_refund(
    State(state): State<AppRefundState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(refund_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match RefundDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &refund_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.retrieve_owner_refund(query).await {
        Ok(Some(refund)) => Json(AppRefundApiResult::success(map_refund(refund))).into_response(),
        Ok(None) => not_found_response("refund was not found"),
        Err(error) => refund_system_response("refund read model is unavailable", error),
    }
}

fn map_refund(value: RefundView) -> RefundResponse {
    RefundResponse {
        refund_id: value.refund_id,
        refund_no: value.refund_no,
        order_id: value.order_id,
        payment_attempt_id: value.payment_attempt_id,
        amount: value.amount.as_str().to_owned(),
        currency_code: value.currency_code,
        status: value.status,
        reason_code: value.reason_code,
    }
}

fn unauthorized_response(message: impl Into<String>) -> Response {
    problem_error_response(StatusCode::UNAUTHORIZED, "4010", message)
}

fn validation_response(message: impl Into<String>) -> Response {
    problem_error_response(StatusCode::BAD_REQUEST, "4001", message)
}

fn not_found_response(message: impl Into<String>) -> Response {
    problem_error_response(StatusCode::NOT_FOUND, "4040", message)
}

fn refund_system_response(context: &str, error: CommerceServiceError) -> Response {
    match error.code() {
        "validation" => validation_response(error.message()),
        "not_found" => not_found_response(error.message()),
        "conflict" => problem_error_response(StatusCode::CONFLICT, "4090", error.message()),
        "unauthenticated" => unauthorized_response(error.message()),
        _ => problem_error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "5000",
            format!("{context}: {}", error.message()),
        ),
    }
}
