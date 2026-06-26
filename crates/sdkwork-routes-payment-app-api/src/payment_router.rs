use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_commerce_contract_service::CommerceServiceError;
use sdkwork_commerce_order_service::{PayOwnerOrderCommand, PayOwnerOrderOutcome};
use sdkwork_commerce_payment_service::{
    ClosePaymentRecordCommand, PaymentMethodItem, PaymentMethodListQuery, PaymentRecordDetailQuery,
    PaymentRecordItem, PaymentRecordListQuery, PaymentRecordOrderListQuery,
};
use sdkwork_commerce_order_repository_sqlx::{PostgresCommerceOrderStore, SqliteCommerceOrderStore};
use sdkwork_commerce_payment_repository_sqlx::{
    PostgresCommerceOwnerOrderPaymentStore, PostgresCommercePaymentRecordStore,
    SqliteCommerceOwnerOrderPaymentStore, SqliteCommercePaymentRecordStore,
};
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::subject::app_runtime_subject_from_extension;

pub type CommercePaymentFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommercePaymentStore: Send + Sync {
    fn list_payment_methods<'a>(
        &'a self,
        query: PaymentMethodListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentMethodItem>>;

    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>>;

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>>;

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()>;

    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome>;
}

#[derive(Clone)]
struct AppPaymentState {
    store: Arc<dyn CommercePaymentStore>,
}

#[derive(Debug, Deserialize)]
struct PaymentRecordsQueryParams {
    page: Option<i64>,
    #[serde(rename = "pageSize", alias = "page_size")]
    page_size: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct PaymentMethodsQueryParams {
    #[serde(rename = "clientType", alias = "client_type")]
    client_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppPaymentApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentRecordsPageResponse {
    content: Vec<PaymentRecordResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentRecordResponse {
    payment_id: String,
    order_id: String,
    out_trade_no: String,
    payment_method: String,
    amount: String,
    created_at: String,
    status: String,
    status_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodResponse {
    method_id: String,
    code: String,
    method_name: String,
    available: bool,
    sort: i64,
    product_types: Vec<PaymentMethodProductTypeResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodProductTypeResponse {
    code: String,
    name: String,
    available: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppCommercePaymentAttemptRecordResponse {
    id: String,
    order_no: String,
    method: String,
    amount: String,
    date: String,
    status: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppCommerceContractApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentStatisticsResponse {
    total_payments: i64,
    pending_payments: i64,
    success_payments: i64,
    failed_payments: i64,
    timeout_payments: i64,
    closed_payments: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReconcilePaymentRequest {
    order_id: Option<String>,
    #[serde(rename = "outTradeNo", alias = "out_trade_no")]
    out_trade_no: Option<String>,
    #[serde(rename = "reconcileType", alias = "reconcile_type")]
    reconcile_type: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaymentRequest {
    order_id: String,
    #[serde(rename = "paymentMethod", alias = "payment_method")]
    payment_method: Option<String>,
    amount: Option<String>,
    #[serde(rename = "businessOrderId", alias = "business_order_id")]
    business_order_id: Option<String>,
    #[serde(rename = "businessType", alias = "business_type")]
    business_type: Option<String>,
    #[serde(rename = "clientIp", alias = "client_ip")]
    client_ip: Option<String>,
    #[serde(rename = "paymentProvider", alias = "payment_provider")]
    payment_provider: Option<String>,
    #[serde(rename = "paymentScene", alias = "payment_scene")]
    payment_scene: Option<String>,
    #[serde(rename = "productType", alias = "product_type")]
    product_type: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreatePaymentResponse {
    payment_id: String,
    order_id: String,
    out_trade_no: String,
    amount: String,
    payment_method: String,
    status: String,
    status_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_params: Option<std::collections::BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_url: Option<String>,
}

impl<T: Serialize> AppPaymentApiResult<T> {
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

pub fn app_payment_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    let order_store = Arc::new(SqliteCommerceOrderStore::new(pool.clone()));
    build_app_payment_router(Arc::new(CompositeCommercePaymentStore {
        methods: order_store,
        payments: Arc::new(SqliteCommerceOwnerOrderPaymentStore::new(pool.clone())),
        records: Arc::new(SqliteCommercePaymentRecordStore::new(pool)),
    }))
}

pub fn app_payment_router_with_postgres_pool(pool: PgPool) -> Router {
    let order_store = Arc::new(PostgresCommerceOrderStore::new(pool.clone()));
    build_app_payment_router(Arc::new(CompositeCommercePaymentStore {
        methods: order_store,
        payments: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
        records: Arc::new(PostgresCommercePaymentRecordStore::new(pool)),
    }))
}

pub fn build_app_payment_router(store: Arc<dyn CommercePaymentStore>) -> Router {
    Router::new()
            .route("/app/v3/api/payments/methods", get(list_payment_methods))
            .route("/app/v3/api/payments/records", get(list_payment_records))
            .route(
                "/app/v3/api/payments/records/{paymentId}",
                get(retrieve_payment_record),
            )
            .route(
                "/app/v3/api/payments/attempts/{paymentAttemptId}",
                get(retrieve_payment_attempt),
            )
            .route(
                "/app/v3/api/payments/statistics",
                get(fetch_payment_statistics),
            )
            .route(
                "/app/v3/api/payments/status/{paymentId}",
                get(retrieve_payment_status),
            )
            .route(
                "/app/v3/api/payments/status/out_trade_no/{outTradeNo}",
                get(retrieve_payment_status_by_out_trade_no),
            )
            .route(
                "/app/v3/api/orders/{orderId}/payments",
                get(list_order_payments),
            )
            .route("/app/v3/api/payments", post(create_payment))
            .route(
                "/app/v3/api/payments/reconciliations",
                post(reconcile_payment),
            )
            .route(
                "/app/v3/api/payments/{paymentId}/close",
                post(close_payment_record),
            )
            .with_state(AppPaymentState { store })
}

struct CompositeCommercePaymentStore {
    methods: Arc<dyn PaymentMethodSource>,
    payments: Arc<dyn OwnerOrderPaymentSource>,
    records: Arc<dyn PaymentRecordSource>,
}

trait PaymentMethodSource: Send + Sync {
    fn list_payment_methods<'a>(
        &'a self,
        query: PaymentMethodListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentMethodItem>>;
}

trait OwnerOrderPaymentSource: Send + Sync {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome>;
}

trait PaymentRecordSource: Send + Sync {
    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>>;

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>>;

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()>;
}

impl PaymentMethodSource for SqliteCommerceOrderStore {
    fn list_payment_methods<'a>(
        &'a self,
        query: PaymentMethodListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentMethodItem>> {
        Box::pin(async move { self.list_payment_methods(query).await })
    }
}

impl OwnerOrderPaymentSource for SqliteCommerceOwnerOrderPaymentStore {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome> {
        Box::pin(async move { self.pay_owner_order(command).await })
    }
}

impl PaymentMethodSource for PostgresCommerceOrderStore {
    fn list_payment_methods<'a>(
        &'a self,
        query: PaymentMethodListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentMethodItem>> {
        Box::pin(async move { self.list_payment_methods(query).await })
    }
}

impl OwnerOrderPaymentSource for PostgresCommerceOwnerOrderPaymentStore {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome> {
        Box::pin(async move { self.pay_owner_order(command).await })
    }
}

impl PaymentRecordSource for SqliteCommercePaymentRecordStore {
    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        Box::pin(async move { self.list_payment_records(query).await })
    }

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        Box::pin(async move { self.retrieve_payment_record(query).await })
    }

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        Box::pin(async move { self.list_payment_records_by_order(query).await })
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        Box::pin(async move { self.close_payment_record(command).await })
    }
}

impl PaymentRecordSource for PostgresCommercePaymentRecordStore {
    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        Box::pin(async move { self.list_payment_records(query).await })
    }

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        Box::pin(async move { self.retrieve_payment_record(query).await })
    }

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        Box::pin(async move { self.list_payment_records_by_order(query).await })
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        Box::pin(async move { self.close_payment_record(command).await })
    }
}

impl CommercePaymentStore for CompositeCommercePaymentStore {
    fn list_payment_methods<'a>(
        &'a self,
        query: PaymentMethodListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentMethodItem>> {
        self.methods.list_payment_methods(query)
    }

    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        self.records.list_payment_records(query)
    }

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        self.records.retrieve_payment_record(query)
    }

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, Vec<PaymentRecordItem>> {
        self.records.list_payment_records_by_order(query)
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        self.records.close_payment_record(command)
    }

    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome> {
        self.payments.pay_owner_order(command)
    }
}

async fn list_payment_methods(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Query(params): Query<PaymentMethodsQueryParams>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query =
        match PaymentMethodListQuery::new(&subject.tenant_id, subject.organization_id.as_deref()) {
            Ok(query) => query,
            Err(error) => return validation_response(error.message()),
        };

    match state.store.list_payment_methods(query).await {
        Ok(items) => {
            let methods = items
                .into_iter()
                .map(map_payment_method)
                .filter(|method| matches_client_type(method, params.client_type.as_deref()))
                .collect::<Vec<_>>();
            Json(AppPaymentApiResult::success(methods)).into_response()
        }
        Err(error) => payment_system_response("payment methods read model is unavailable", error),
    }
}

async fn list_payment_records(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Query(params): Query<PaymentRecordsQueryParams>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match PaymentRecordListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.list_payment_records(query).await {
        Ok(items) => {
            let content = paginate_items(
                items
                    .into_iter()
                    .map(map_payment_record)
                    .collect::<Vec<_>>(),
                params.page,
                params.page_size,
            );
            Json(AppPaymentApiResult::success(PaymentRecordsPageResponse {
                content,
            }))
            .into_response()
        }
        Err(error) => payment_system_response("payment records read model is unavailable", error),
    }
}

async fn list_order_payments(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(order_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match PaymentRecordOrderListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.list_payment_records_by_order(query).await {
        Ok(items) => Json(AppPaymentApiResult::success(
            items
                .into_iter()
                .map(map_payment_record)
                .collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => payment_system_response("order payment read model is unavailable", error),
    }
}

async fn retrieve_payment_record(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_id): Path<String>,
) -> Response {
    match load_payment_record(state, runtime_context, payment_id).await {
        Ok(record) => {
            Json(AppPaymentApiResult::success(map_payment_record(record))).into_response()
        }
        Err(response) => response,
    }
}

async fn retrieve_payment_attempt(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_attempt_id): Path<String>,
) -> Response {
    match load_payment_record(state, runtime_context, payment_attempt_id).await {
        Ok(record) => Json(AppCommerceContractApiResult::success(
            map_payment_attempt_record(record),
        ))
        .into_response(),
        Err(response) => response,
    }
}

async fn retrieve_payment_status(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_id): Path<String>,
) -> Response {
    match load_payment_record(state, runtime_context, payment_id).await {
        Ok(record) => {
            Json(AppPaymentApiResult::success(map_payment_record(record))).into_response()
        }
        Err(response) => response,
    }
}

async fn retrieve_payment_status_by_out_trade_no(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(out_trade_no): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match PaymentRecordListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.list_payment_records(query).await {
        Ok(items) => {
            let Some(record) = items.into_iter().find(|item| item.order_no == out_trade_no) else {
                return not_found_response("payment record was not found");
            };
            Json(AppPaymentApiResult::success(map_payment_record(record))).into_response()
        }
        Err(error) => payment_system_response("payment records read model is unavailable", error),
    }
}

async fn fetch_payment_statistics(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = match PaymentRecordListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.list_payment_records(query).await {
        Ok(items) => {
            Json(AppPaymentApiResult::success(map_payment_statistics(items))).into_response()
        }
        Err(error) => {
            payment_system_response("payment statistics read model is unavailable", error)
        }
    }
}

async fn load_payment_record(
    state: AppPaymentState,
    runtime_context: Option<Extension<IamAppContext>>,
    payment_id: String,
) -> Result<PaymentRecordItem, Response> {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return Err(unauthorized_response(message)),
    };
    let query = match PaymentRecordDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_id,
    ) {
        Ok(query) => query,
        Err(error) => return Err(validation_response(error.message())),
    };

    state
        .store
        .retrieve_payment_record(query)
        .await
        .map_err(|error| payment_system_response("payment record read model is unavailable", error))
}

async fn reconcile_payment(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    body: Json<ReconcilePaymentRequest>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let order_id = body
        .order_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let out_trade_no = body
        .out_trade_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let reconcile_type = body
        .reconcile_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            if order_id.is_some() {
                "ORDER_ID"
            } else {
                "OUT_TRADE_NO"
            }
        });

    let record = if reconcile_type.eq_ignore_ascii_case("ORDER_ID") {
        let Some(order_id) = order_id else {
            return validation_response("orderId is required for ORDER_ID reconciliation");
        };
        let query = match PaymentRecordListQuery::new(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
        ) {
            Ok(query) => query,
            Err(error) => return validation_response(error.message()),
        };
        match state.store.list_payment_records(query).await {
            Ok(items) => items.into_iter().find(|item| item.order_id == order_id),
            Err(error) => {
                return payment_system_response(
                    "payment reconcile read model is unavailable",
                    error,
                )
            }
        }
    } else if out_trade_no.is_some() || reconcile_type.eq_ignore_ascii_case("OUT_TRADE_NO") {
        let Some(out_trade_no) = out_trade_no else {
            return validation_response("outTradeNo is required for OUT_TRADE_NO reconciliation");
        };
        let query = match PaymentRecordListQuery::new(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
        ) {
            Ok(query) => query,
            Err(error) => return validation_response(error.message()),
        };
        match state.store.list_payment_records(query).await {
            Ok(items) => items.into_iter().find(|item| item.order_no == out_trade_no),
            Err(error) => {
                return payment_system_response(
                    "payment reconcile read model is unavailable",
                    error,
                )
            }
        }
    } else {
        return validation_response("reconcileType must be ORDER_ID or OUT_TRADE_NO");
    };

    let Some(record) = record else {
        return not_found_response("payment record was not found");
    };

    Json(AppPaymentApiResult::success(map_payment_record(record))).into_response()
}

async fn create_payment(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    body: Json<CreatePaymentRequest>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let payment_method = body
        .payment_method
        .clone()
        .unwrap_or_else(|| "wechat_pay".to_owned());
    let _ = (
        body.amount.as_deref(),
        body.business_order_id.as_deref(),
        body.business_type.as_deref(),
        body.client_ip.as_deref(),
        body.payment_provider.as_deref(),
        body.payment_scene.as_deref(),
        body.product_type.as_deref(),
    );
    let command = match PayOwnerOrderCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &body.order_id,
        &payment_method,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.pay_owner_order(command).await {
        Ok(outcome) => {
            Json(AppPaymentApiResult::success(map_create_payment(outcome))).into_response()
        }
        Err(error) => payment_system_response("payment create command failed", error),
    }
}

async fn close_payment_record(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let command = match ClosePaymentRecordCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_id,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(error.message()),
    };

    match state.store.close_payment_record(command).await {
        Ok(()) => (
            StatusCode::OK,
            Json(AppPaymentApiResult::<()> {
                code: "0".to_owned(),
                msg: "success".to_owned(),
                data: None,
            }),
        )
            .into_response(),
        Err(error) => payment_system_response("payment close command failed", error),
    }
}

fn paginate_items<T>(items: Vec<T>, page: Option<i64>, page_size: Option<i64>) -> Vec<T> {
    let page = page.unwrap_or(1).max(1);
    let page_size = page_size.unwrap_or(20).clamp(1, 100);
    let start = ((page - 1) * page_size) as usize;
    items
        .into_iter()
        .skip(start)
        .take(page_size as usize)
        .collect()
}

fn matches_client_type(method: &PaymentMethodResponse, client_type: Option<&str>) -> bool {
    let Some(client_type) = client_type.map(str::trim) else {
        return true;
    };
    if client_type.is_empty() {
        return true;
    }

    method
        .product_types
        .iter()
        .any(|product| product.available && product.code.eq_ignore_ascii_case(client_type))
}

fn map_create_payment(value: PayOwnerOrderOutcome) -> CreatePaymentResponse {
    let payment_url = value
        .payment_params
        .get("cashierUrl")
        .cloned()
        .or_else(|| value.payment_params.get("paymentUrl").cloned());
    CreatePaymentResponse {
        payment_id: value.payment_id,
        order_id: value.order_id,
        out_trade_no: value.out_trade_no,
        amount: value.amount.as_str().to_owned(),
        payment_method: value.payment_method,
        status: "PENDING".to_owned(),
        status_name: format_payment_status_name("PENDING"),
        payment_params: Some(value.payment_params),
        payment_url,
    }
}

fn map_payment_method(value: PaymentMethodItem) -> PaymentMethodResponse {
    PaymentMethodResponse {
        method_id: value.id,
        code: value.method_key,
        method_name: value.display_name,
        available: true,
        sort: value.sort_order,
        product_types: vec![PaymentMethodProductTypeResponse {
            code: "pc".to_owned(),
            name: "PC".to_owned(),
            available: true,
        }],
    }
}

fn map_payment_record(value: PaymentRecordItem) -> PaymentRecordResponse {
    let status = map_payment_status_code(&value.status);
    PaymentRecordResponse {
        payment_id: value.id,
        order_id: value.order_id,
        out_trade_no: value.order_no,
        payment_method: value.method,
        amount: value.amount.as_str().to_owned(),
        created_at: value.date,
        status: status.to_owned(),
        status_name: format_payment_status_name(status),
    }
}

impl<T: Serialize> AppCommerceContractApiResult<T> {
    fn success(data: T) -> Self {
        Self {
            code: "2000".to_owned(),
            msg: "SUCCESS".to_owned(),
            data: Some(data),
        }
    }
}

fn map_payment_attempt_record(value: PaymentRecordItem) -> AppCommercePaymentAttemptRecordResponse {
    AppCommercePaymentAttemptRecordResponse {
        id: value.id,
        order_no: value.order_no,
        method: value.method,
        amount: value.amount.as_str().to_owned(),
        date: value.date,
        status: value.status,
    }
}

fn map_payment_statistics(items: Vec<PaymentRecordItem>) -> PaymentStatisticsResponse {
    let mut statistics = PaymentStatisticsResponse {
        total_payments: items.len() as i64,
        pending_payments: 0,
        success_payments: 0,
        failed_payments: 0,
        timeout_payments: 0,
        closed_payments: 0,
    };

    for item in items {
        match map_payment_status_code(&item.status) {
            "PENDING" => statistics.pending_payments += 1,
            "SUCCESS" => statistics.success_payments += 1,
            "FAILED" => statistics.failed_payments += 1,
            "TIMEOUT" => statistics.timeout_payments += 1,
            "CLOSED" => statistics.closed_payments += 1,
            _ => statistics.pending_payments += 1,
        }
    }

    statistics
}

fn map_payment_status_code(status: &str) -> &'static str {
    match status.trim().to_ascii_lowercase().as_str() {
        "success" => "SUCCESS",
        "failed" => "FAILED",
        "timeout" => "TIMEOUT",
        "closed" => "CLOSED",
        _ => "PENDING",
    }
}

fn format_payment_status_name(status: &str) -> String {
    match status {
        "SUCCESS" => "Success".to_owned(),
        "FAILED" => "Failed".to_owned(),
        "TIMEOUT" => "Timeout".to_owned(),
        "CLOSED" => "Closed".to_owned(),
        _ => "Pending".to_owned(),
    }
}

fn payment_system_response(context: &str, error: CommerceServiceError) -> Response {
    let _ = context;
    match error.code() {
        "validation" => validation_response(error.message()),
        "unauthenticated" | "unauthorized" => unauthorized_response(error.message().to_owned()),
        "not-found" => not_found_response(error.message()),
        "conflict" => (
            StatusCode::CONFLICT,
            Json(AppPaymentApiResult::<()>::error("4090", error.message())),
        )
            .into_response(),
        _ => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(AppPaymentApiResult::<()>::error("5000", error.message())),
        )
            .into_response(),
    }
}

fn unauthorized_response(message: String) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(AppPaymentApiResult::<()>::error("4010", message)),
    )
        .into_response()
}

fn validation_response(message: impl Into<String>) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(AppPaymentApiResult::<()>::error("4001", message)),
    )
        .into_response()
}

fn not_found_response(message: impl Into<String>) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(AppPaymentApiResult::<()>::error("4040", message)),
    )
        .into_response()
}
