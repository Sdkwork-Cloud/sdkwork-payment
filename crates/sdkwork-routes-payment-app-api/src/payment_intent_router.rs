use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_providers::{PaymentProviderRegistry, ProviderCredentialBundle};
use sdkwork_payment_service::{
    CancelOwnerPaymentIntentCommand, CreateOwnerPaymentAttemptCommand,
    CreateOwnerPaymentAttemptOutcome, CreateOwnerPaymentIntentCommand, PaymentIntentDetailQuery,
    PaymentIntentView,
};
use sdkwork_payment_repository_sqlx::{
    enrich_owner_payment_attempt_postgres, enrich_owner_payment_attempt_sqlite,
    PostgresCommercePaymentIntentStore, SqliteCommercePaymentIntentStore,
};
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{
    map_service_error, not_found, success_command_accepted, success_item, unauthorized,
    validation,
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

struct ProviderEnrichedSqlitePaymentIntents {
    inner: Arc<SqliteCommercePaymentIntentStore>,
    pool: SqlitePool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
}

struct ProviderEnrichedPostgresPaymentIntents {
    inner: Arc<PostgresCommercePaymentIntentStore>,
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
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

impl CommercePaymentIntentStore for ProviderEnrichedSqlitePaymentIntents {
    fn create_owner_payment_intent<'a>(
        &'a self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.create_owner_payment_intent(command).await })
    }

    fn retrieve_owner_payment_intent<'a>(
        &'a self,
        query: PaymentIntentDetailQuery,
    ) -> CommercePaymentIntentFuture<'a, Option<PaymentIntentView>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_owner_payment_intent(query).await })
    }

    fn cancel_owner_payment_intent<'a>(
        &'a self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.cancel_owner_payment_intent(command).await })
    }

    fn create_owner_payment_attempt<'a>(
        &'a self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> CommercePaymentIntentFuture<'a, CreateOwnerPaymentAttemptOutcome> {
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        let pool = self.pool.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let tenant_id = command.tenant_id.clone();
            let organization_id = command.organization_id.clone();
            let idempotency_key = command.idempotency_key.clone();
            let outcome = inner.create_owner_payment_attempt(command).await?;
            let order_id = outcome.order_id.clone();
            enrich_owner_payment_attempt_sqlite(
                &pool,
                &registry,
                &credentials,
                &tenant_id,
                organization_id.as_deref(),
                &order_id,
                &idempotency_key,
                None,
                outcome,
            )
            .await
        })
    }
}

impl CommercePaymentIntentStore for ProviderEnrichedPostgresPaymentIntents {
    fn create_owner_payment_intent<'a>(
        &'a self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.create_owner_payment_intent(command).await })
    }

    fn retrieve_owner_payment_intent<'a>(
        &'a self,
        query: PaymentIntentDetailQuery,
    ) -> CommercePaymentIntentFuture<'a, Option<PaymentIntentView>> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_owner_payment_intent(query).await })
    }

    fn cancel_owner_payment_intent<'a>(
        &'a self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> CommercePaymentIntentFuture<'a, PaymentIntentView> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.cancel_owner_payment_intent(command).await })
    }

    fn create_owner_payment_attempt<'a>(
        &'a self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> CommercePaymentIntentFuture<'a, CreateOwnerPaymentAttemptOutcome> {
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        let pool = self.pool.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            let tenant_id = command.tenant_id.clone();
            let organization_id = command.organization_id.clone();
            let idempotency_key = command.idempotency_key.clone();
            let outcome = inner.create_owner_payment_attempt(command).await?;
            let order_id = outcome.order_id.clone();
            enrich_owner_payment_attempt_postgres(
                &pool,
                &registry,
                &credentials,
                &tenant_id,
                organization_id.as_deref(),
                &order_id,
                &idempotency_key,
                None,
                outcome,
            )
            .await
        })
    }
}

pub fn app_payment_intent_router_with_sqlite_pool(
    pool: SqlitePool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    build_app_payment_intent_router(Arc::new(ProviderEnrichedSqlitePaymentIntents {
        inner: Arc::new(SqliteCommercePaymentIntentStore::new(pool.clone())),
        pool,
        registry,
        credentials,
    }))
}

pub fn app_payment_intent_router_with_postgres_pool(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    build_app_payment_intent_router(Arc::new(ProviderEnrichedPostgresPaymentIntents {
        inner: Arc::new(PostgresCommercePaymentIntentStore::new(pool.clone())),
        pool,
        registry,
        credentials,
    }))
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
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    body: Json<CreatePaymentIntentRequest>,
) -> Response {
    let ctx = request_ctx(&request_context);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
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
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.create_owner_payment_intent(command).await {
        Ok(intent) => success_command_accepted(ctx, Some(intent.payment_intent_id)),
        Err(error) => payment_intent_system_response(ctx, "payment intent create failed", error),
    }
}

async fn retrieve_payment_intent(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let ctx = request_ctx(&request_context);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match PaymentIntentDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_intent_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.retrieve_owner_payment_intent(query).await {
        Ok(Some(intent)) => success_item(ctx, map_payment_intent(intent)),
        Ok(None) => not_found(ctx, "payment intent was not found"),
        Err(error) => {
            payment_intent_system_response(ctx, "payment intent read model is unavailable", error)
        }
    }
}

async fn cancel_payment_intent(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let ctx = request_ctx(&request_context);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
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
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.cancel_owner_payment_intent(command).await {
        Ok(_intent) => success_command_accepted(ctx, Some(payment_intent_id)),
        Err(error) => payment_intent_system_response(ctx, "payment intent cancel failed", error),
    }
}

async fn create_payment_attempt(
    State(state): State<AppPaymentIntentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(payment_intent_id): Path<String>,
) -> Response {
    let ctx = request_ctx(&request_context);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
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
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.create_owner_payment_attempt(command).await {
        Ok(outcome) => success_command_accepted(ctx, Some(outcome.attempt_id)),
        Err(error) => payment_intent_system_response(ctx, "payment attempt create failed", error),
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

fn request_ctx(ext: &Option<Extension<WebRequestContext>>) -> Option<&WebRequestContext> {
    ext.as_ref().map(|Extension(value)| value)
}

fn payment_intent_system_response(
    context: Option<&WebRequestContext>,
    _label: &str,
    error: CommerceServiceError,
) -> Response {
    map_service_error(context, error)
}
