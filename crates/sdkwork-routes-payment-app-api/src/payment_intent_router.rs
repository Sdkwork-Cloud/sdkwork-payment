use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_service::{
    CancelOwnerPaymentIntentCommand, CreateOwnerPaymentAttemptCommand,
    CreateOwnerPaymentAttemptOutcome, CreateOwnerPaymentIntentCommand, PaymentIntentDetailQuery,
    PaymentIntentView,
};
use sdkwork_payment_repository_sqlx::{
    PostgresCommercePaymentIntentStore, SqliteCommercePaymentIntentStore,
};
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{
    map_service_error, not_found, success_item, unauthorized, validation,
};
use crate::command_headers::{validate_app_write_payload, write_payload_with_route_param};
use crate::subject::app_runtime_subject_from_extension;

pub type CommercePaymentIntentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommercePaymentIntentStore: Send + Sync {
    fn create_owner_payment_intent<'a>(
        &'a self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView>;

    fn retrieve_owner_payment_intent<'a>(
        &'a self,
        query: PaymentIntentDetailQuery,
    ) -> CommercePaymentIntentFuture<'a, Option<PaymentIntentView>>;

    fn cancel_owner_payment_intent<'a>(
        &'a self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView>;

    fn create_owner_payment_attempt<'a>(
        &'a self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> CommercePaymentIntentFuture<'a, CreateOwnerPaymentAttemptOutcome>;
}

#[derive(Clone)]
struct AppPaymentIntentState {
    store: Arc<dyn CommercePaymentIntentStore>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaymentIntentRequest {
    order_id: String,
    #[serde(rename = "paymentMethod", alias = "payment_method")]
    payment_method: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentIntentResponse {
    payment_intent_id: String,
    order_id: String,
    payment_intent_no: String,
    payment_method: String,
    provider_code: String,
    amount: String,
    currency_code: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentAttemptResponse {
    attempt_id: String,
    payment_intent_id: String,
    order_id: String,
    out_trade_no: String,
    amount: String,
    payment_method: String,
    status: String,
}

impl CommercePaymentIntentStore for SqliteCommercePaymentIntentStore {
    fn create_owner_payment_intent<'a>(
        &'a self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        Box::pin(async move { self.create_owner_payment_intent(command).await })
    }

    fn retrieve_owner_payment_intent<'a>(
        &'a self,
        query: PaymentIntentDetailQuery,
    ) -> CommercePaymentIntentFuture<'a, Option<PaymentIntentView>> {
        Box::pin(async move { self.retrieve_owner_payment_intent(query).await })
    }

    fn cancel_owner_payment_intent<'a>(
        &'a self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        Box::pin(async move { self.cancel_owner_payment_intent(command).await })
    }

    fn create_owner_payment_attempt<'a>(
        &'a self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> CommercePaymentIntentFuture<'a, CreateOwnerPaymentAttemptOutcome> {
        Box::pin(async move { self.create_owner_payment_attempt(command).await })
    }
}

impl CommercePaymentIntentStore for PostgresCommercePaymentIntentStore {
    fn create_owner_payment_intent<'a>(
        &'a self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        Box::pin(async move { self.create_owner_payment_intent(command).await })
    }

    fn retrieve_owner_payment_intent<'a>(
        &'a self,
        query: PaymentIntentDetailQuery,
    ) -> CommercePaymentIntentFuture<'a, Option<PaymentIntentView>> {
        Box::pin(async move { self.retrieve_owner_payment_intent(query).await })
    }

    fn cancel_owner_payment_intent<'a>(
        &'a self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        Box::pin(async move { self.cancel_owner_payment_intent(command).await })
    }

    fn create_owner_payment_attempt<'a>(
        &'a self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> CommercePaymentIntentFuture<'a, CreateOwnerPaymentAttemptOutcome> {
        Box::pin(async move { self.create_owner_payment_attempt(command).await })
    }
}

pub fn app_payment_intent_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_app_payment_intent_router(Arc::new(SqliteCommercePaymentIntentStore::new(pool)))
}

pub fn app_payment_intent_router_with_postgres_pool(pool: PgPool) -> Router {
    build_app_payment_intent_router(Arc::new(PostgresCommercePaymentIntentStore::new(pool)))
}

pub fn build_app_payment_intent_router(store: Arc<dyn CommercePaymentIntentStore>) -> Router {
    Router::new()
            .route("/app/v3/api/payments/intents", post(create_payment_intent))
            .route(
                "/app/v3/api/payments/intents/{paymentIntentId}",
                get(retrieve_payment_intent),
            )
            .route(
                "/app/v3/api/payments/intents/{paymentIntentId}/cancel",
                post(cancel_payment_intent),
            )
            .route(
                "/app/v3/api/payments/intents/{paymentIntentId}/attempts",
                post(create_payment_attempt),
            )
            .with_state(AppPaymentIntentState { store })
}

async fn create_payment_intent(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    body: Json<CreatePaymentIntentRequest>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let write_headers = match validate_app_write_payload(
        &headers,
        "payments.intents.create",
        &*body,
        |idempotency_key| format!("payment-intent-{}-{}", subject.user_id, idempotency_key),
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let payment_method = body
        .payment_method
        .clone()
        .unwrap_or_else(|| "wechat_pay".to_owned());
    let command = match CreateOwnerPaymentIntentCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &body.order_id,
        &payment_method,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.create_owner_payment_intent(command).await {
        Ok(intent) => success_item(None, map_payment_intent(intent)),
        Err(error) => payment_intent_system_response("payment intent create failed", error),
    }
}

async fn retrieve_payment_intent(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match PaymentIntentDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_intent_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.retrieve_owner_payment_intent(query).await {
        Ok(Some(intent)) => success_item(None, map_payment_intent(intent)),
        Ok(None) => not_found(None, "payment intent was not found"),
        Err(error) => {
            payment_intent_system_response("payment intent read model is unavailable", error)
        }
    }
}

async fn cancel_payment_intent(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let payload =
        write_payload_with_route_param("paymentIntentId", &payment_intent_id, &serde_json::json!({}));
    let _write_headers = match validate_app_write_payload(
        &headers,
        "payments.intents.cancel",
        &payload,
        |idempotency_key| {
            format!(
                "payment-intent-cancel-{}-{}",
                subject.user_id, idempotency_key
            )
        },
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let command = match CancelOwnerPaymentIntentCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_intent_id,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.cancel_owner_payment_intent(command).await {
        Ok(intent) => success_item(None, map_payment_intent(intent)),
        Err(error) => payment_intent_system_response("payment intent cancel failed", error),
    }
}

async fn create_payment_attempt(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let payload = serde_json::json!({ "paymentIntentId": payment_intent_id });
    let write_headers = match validate_app_write_payload(
        &headers,
        "payments.attempts.create",
        &payload,
        |idempotency_key| format!("payment-attempt-{}-{}", subject.user_id, idempotency_key),
    ) {
        Ok(value) => value,
        Err(response) => return response,
    };
    let command = match CreateOwnerPaymentAttemptCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_intent_id,
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.create_owner_payment_attempt(command).await {
        Ok(outcome) => success_item(None, map_payment_attempt(outcome)),
        Err(error) => payment_intent_system_response("payment attempt create failed", error),
    }
}

fn map_payment_intent(value: PaymentIntentView) -> PaymentIntentResponse {
    PaymentIntentResponse {
        payment_intent_id: value.payment_intent_id,
        order_id: value.order_id,
        payment_intent_no: value.payment_intent_no,
        payment_method: value.payment_method,
        provider_code: value.provider_code,
        amount: value.amount.as_str().to_owned(),
        currency_code: value.currency_code,
        status: value.status,
    }
}

fn map_payment_attempt(value: CreateOwnerPaymentAttemptOutcome) -> PaymentAttemptResponse {
    PaymentAttemptResponse {
        attempt_id: value.attempt_id,
        payment_intent_id: value.payment_intent_id,
        order_id: value.order_id,
        out_trade_no: value.out_trade_no,
        amount: value.amount.as_str().to_owned(),
        payment_method: value.payment_method,
        status: value.status,
    }
}

fn unauthorized_response(message: impl Into<String>) -> Response {
    unauthorized(None, message)
}

fn validation_response(message: impl Into<String>) -> Response {
    validation(None, message)
}

fn payment_intent_system_response(_context: &str, error: CommerceServiceError) -> Response {
    map_service_error(None, error)
}
