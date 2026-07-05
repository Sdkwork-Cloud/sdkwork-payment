use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_order_service::{PayOwnerOrderCommand, PayOwnerOrderOutcome};
use sdkwork_payment_service::{
    ClosePaymentRecordCommand, PaymentMethodItem, PaymentMethodListQuery, PaymentRecordDetailQuery,
    PaymentRecordItem, PaymentRecordListPage, PaymentRecordListQuery, PaymentRecordOrderListPage,
    PaymentRecordOrderListQuery, PaymentRecordOutTradeNoQuery, PaymentRecordStatistics,
    PaymentRecordStatisticsQuery,
};
use sdkwork_order_repository_sqlx::{PostgresCommerceOrderStore, SqliteCommerceOrderStore};
use sdkwork_payment_providers::{
    cancel_provider_payment, enrich_pay_owner_order_outcome, normalize_provider_code,
    provider_registry_for_account, CheckoutContext, PaymentProviderRegistry,
    ProviderAccountBinding, ProviderCredentialBundle,
};
use sdkwork_payment_repository_sqlx::{
    load_active_provider_account_postgres, load_active_provider_account_sqlite,
    load_payment_attempt_provider_context_postgres, load_payment_attempt_provider_context_sqlite,
    PaymentProviderAccountRecord, PostgresCommerceOwnerOrderPaymentStore,
    PostgresCommercePaymentRecordStore, SqliteCommerceOwnerOrderPaymentStore,
    SqliteCommercePaymentRecordStore,
};
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_utils_rust::OffsetListPageParams;
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, SqlitePool};

use crate::api_response::{
    map_service_error, not_found, success_command_accepted, success_item, success_list,
    unauthorized, validation,
};
use crate::command_headers::{validate_app_write_payload, write_payload_with_route_param};
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
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage>;

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage>;

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics>;

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
struct OrderPaymentsQueryParams {
    page: Option<i64>,
    #[serde(rename = "pageSize", alias = "page_size")]
    page_size: Option<i64>,
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
struct PaymentStatisticsResponse {
    total_payments: i64,
    pending_payments: i64,
    success_payments: i64,
    failed_payments: i64,
    timeout_payments: i64,
    closed_payments: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReconcilePaymentRequest {
    order_id: Option<String>,
    #[serde(rename = "outTradeNo", alias = "out_trade_no")]
    out_trade_no: Option<String>,
    #[serde(rename = "reconcileType", alias = "reconcile_type")]
    reconcile_type: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
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

pub fn app_payment_router_with_sqlite_pool(
    pool: SqlitePool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    let order_store = Arc::new(SqliteCommerceOrderStore::new(pool.clone()));
    build_app_payment_router(Arc::new(CompositeCommercePaymentStore {
        methods: order_store,
        payments: Arc::new(ProviderEnrichedSqlitePayments {
            inner: Arc::new(SqliteCommerceOwnerOrderPaymentStore::new(pool.clone())),
            pool: pool.clone(),
            registry,
            credentials: credentials.clone(),
        }),
        records: Arc::new(ProviderEnrichedSqlitePaymentRecords {
            inner: Arc::new(SqliteCommercePaymentRecordStore::new(pool.clone())),
            pool,
            credentials,
        }),
    }))
}

pub fn app_payment_router_with_postgres_pool(
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
) -> Router {
    let order_store = Arc::new(PostgresCommerceOrderStore::new(pool.clone()));
    build_app_payment_router(Arc::new(CompositeCommercePaymentStore {
        methods: order_store,
        payments: Arc::new(ProviderEnrichedPostgresPayments {
            inner: Arc::new(PostgresCommerceOwnerOrderPaymentStore::new(pool.clone())),
            pool: pool.clone(),
            registry,
            credentials: credentials.clone(),
        }),
        records: Arc::new(ProviderEnrichedPostgresPaymentRecords {
            inner: Arc::new(PostgresCommercePaymentRecordStore::new(pool.clone())),
            pool,
            credentials,
        }),
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

struct ProviderEnrichedSqlitePayments {
    inner: Arc<SqliteCommerceOwnerOrderPaymentStore>,
    pool: SqlitePool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
}

struct ProviderEnrichedPostgresPayments {
    inner: Arc<PostgresCommerceOwnerOrderPaymentStore>,
    pool: PgPool,
    registry: Arc<PaymentProviderRegistry>,
    credentials: ProviderCredentialBundle,
}

struct ProviderEnrichedSqlitePaymentRecords {
    inner: Arc<SqliteCommercePaymentRecordStore>,
    pool: SqlitePool,
    credentials: ProviderCredentialBundle,
}

struct ProviderEnrichedPostgresPaymentRecords {
    inner: Arc<PostgresCommercePaymentRecordStore>,
    pool: PgPool,
    credentials: ProviderCredentialBundle,
}

fn provider_account_binding(record: &PaymentProviderAccountRecord) -> ProviderAccountBinding {
    ProviderAccountBinding {
        provider_code: record.provider_code.clone(),
        merchant_id: record.merchant_id.clone(),
        environment: record.environment.clone(),
        secret_ref: record.secret_ref.clone(),
        webhook_secret_ref: record.webhook_secret_ref.clone(),
        certificate_ref: record.certificate_ref.clone(),
        metadata: record.metadata.clone(),
    }
}

async fn enrich_owner_order_payment(
    pool: &SqlitePool,
    registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account = load_active_provider_account_sqlite(
        pool,
        tenant_id,
        organization_id,
        &provider_code,
    )
    .await?;
    let tenant_registry = tenant_provider_registry(registry, credentials, account);
    let notify_url = credentials.provider_notify_url(&normalize_provider_code(&provider_code));
    let context = CheckoutContext {
        provider_code,
        currency_code: "CNY".to_owned(),
        tenant_id: tenant_id.to_owned(),
        order_id: order_id.to_owned(),
        idempotency_key: idempotency_key.to_owned(),
        notify_url,
        payment_scene: None,
    };
    enrich_pay_owner_order_outcome(&tenant_registry, &context, outcome).await
}

async fn enrich_owner_order_payment_postgres(
    pool: &PgPool,
    registry: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    tenant_id: &str,
    organization_id: Option<&str>,
    order_id: &str,
    idempotency_key: &str,
    outcome: PayOwnerOrderOutcome,
) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
    let provider_code = outcome
        .payment_params
        .get("providerCode")
        .cloned()
        .unwrap_or_else(|| "sandbox".to_owned());
    let account = load_active_provider_account_postgres(
        pool,
        tenant_id,
        organization_id,
        &provider_code,
    )
    .await?;
    let tenant_registry = tenant_provider_registry(registry, credentials, account);
    let notify_url = credentials.provider_notify_url(&normalize_provider_code(&provider_code));
    let context = CheckoutContext {
        provider_code,
        currency_code: "CNY".to_owned(),
        tenant_id: tenant_id.to_owned(),
        order_id: order_id.to_owned(),
        idempotency_key: idempotency_key.to_owned(),
        notify_url,
        payment_scene: None,
    };
    enrich_pay_owner_order_outcome(&tenant_registry, &context, outcome).await
}

fn tenant_provider_registry(
    fallback: &PaymentProviderRegistry,
    credentials: &ProviderCredentialBundle,
    account: Option<PaymentProviderAccountRecord>,
) -> PaymentProviderRegistry {
    match account {
        Some(record) => provider_registry_for_account(
            credentials,
            Some(provider_account_binding(&record)),
        ),
        None => fallback.clone(),
    }
}

impl OwnerOrderPaymentSource for ProviderEnrichedSqlitePayments {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome> {
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let tenant_id = command.tenant_id.clone();
            let organization_id = command.organization_id.clone();
            let order_id = command.order_id.clone();
            let idempotency_key = command.idempotency_key.clone();
            let outcome = self.inner.pay_owner_order(command).await?;
            enrich_owner_order_payment(
                &pool,
                &registry,
                &credentials,
                &tenant_id,
                organization_id.as_deref(),
                &order_id,
                &idempotency_key,
                outcome,
            )
            .await
        })
    }
}

impl OwnerOrderPaymentSource for ProviderEnrichedPostgresPayments {
    fn pay_owner_order<'a>(
        &'a self,
        command: PayOwnerOrderCommand,
    ) -> CommercePaymentFuture<'a, PayOwnerOrderOutcome> {
        let registry = self.registry.clone();
        let credentials = self.credentials.clone();
        let pool = self.pool.clone();
        Box::pin(async move {
            let tenant_id = command.tenant_id.clone();
            let organization_id = command.organization_id.clone();
            let order_id = command.order_id.clone();
            let idempotency_key = command.idempotency_key.clone();
            let outcome = self.inner.pay_owner_order(command).await?;
            enrich_owner_order_payment_postgres(
                &pool,
                &registry,
                &credentials,
                &tenant_id,
                organization_id.as_deref(),
                &order_id,
                &idempotency_key,
                outcome,
            )
            .await
        })
    }
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
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage>;

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage>;

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem>;

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics>;

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
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage> {
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
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage> {
        Box::pin(async move { self.list_payment_records_by_order(query).await })
    }

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        Box::pin(async move { self.retrieve_payment_record_by_out_trade_no(query).await })
    }

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics> {
        Box::pin(async move { self.fetch_payment_statistics(query).await })
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
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage> {
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
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage> {
        Box::pin(async move { self.list_payment_records_by_order(query).await })
    }

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        Box::pin(async move { self.retrieve_payment_record_by_out_trade_no(query).await })
    }

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics> {
        Box::pin(async move { self.fetch_payment_statistics(query).await })
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        Box::pin(async move { self.close_payment_record(command).await })
    }
}

impl PaymentRecordSource for ProviderEnrichedSqlitePaymentRecords {
    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.list_payment_records(query).await })
    }

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_payment_record(query).await })
    }

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.list_payment_records_by_order(query).await })
    }

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_payment_record_by_out_trade_no(query).await })
    }

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.fetch_payment_statistics(query).await })
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        let pool = self.pool.clone();
        let credentials = self.credentials.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            if let Some(ctx) = load_payment_attempt_provider_context_sqlite(
                &pool,
                &command.tenant_id,
                &command.owner_user_id,
                &command.payment_id,
            )
            .await?
            {
                let account = load_active_provider_account_sqlite(
                    &pool,
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &ctx.provider_code,
                )
                .await?;
                let registry = provider_registry_for_account(
                    &credentials,
                    account.map(|record| provider_account_binding(&record)),
                );
                cancel_provider_payment(&registry, &ctx.provider_code, &ctx.out_trade_no).await?;
            }
            inner.close_payment_record(command).await
        })
    }
}

impl PaymentRecordSource for ProviderEnrichedPostgresPaymentRecords {
    fn list_payment_records<'a>(
        &'a self,
        query: PaymentRecordListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.list_payment_records(query).await })
    }

    fn retrieve_payment_record<'a>(
        &'a self,
        query: PaymentRecordDetailQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_payment_record(query).await })
    }

    fn list_payment_records_by_order<'a>(
        &'a self,
        query: PaymentRecordOrderListQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.list_payment_records_by_order(query).await })
    }

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.retrieve_payment_record_by_out_trade_no(query).await })
    }

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics> {
        let inner = self.inner.clone();
        Box::pin(async move { inner.fetch_payment_statistics(query).await })
    }

    fn close_payment_record<'a>(
        &'a self,
        command: ClosePaymentRecordCommand,
    ) -> CommercePaymentFuture<'a, ()> {
        let pool = self.pool.clone();
        let credentials = self.credentials.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            if let Some(ctx) = load_payment_attempt_provider_context_postgres(
                &pool,
                &command.tenant_id,
                &command.owner_user_id,
                &command.payment_id,
            )
            .await?
            {
                let account = load_active_provider_account_postgres(
                    &pool,
                    &command.tenant_id,
                    command.organization_id.as_deref(),
                    &ctx.provider_code,
                )
                .await?;
                let registry = provider_registry_for_account(
                    &credentials,
                    account.map(|record| provider_account_binding(&record)),
                );
                cancel_provider_payment(&registry, &ctx.provider_code, &ctx.out_trade_no).await?;
            }
            inner.close_payment_record(command).await
        })
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
    ) -> CommercePaymentFuture<'a, PaymentRecordListPage> {
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
    ) -> CommercePaymentFuture<'a, PaymentRecordOrderListPage> {
        self.records.list_payment_records_by_order(query)
    }

    fn retrieve_payment_record_by_out_trade_no<'a>(
        &'a self,
        query: PaymentRecordOutTradeNoQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordItem> {
        self.records.retrieve_payment_record_by_out_trade_no(query)
    }

    fn fetch_payment_statistics<'a>(
        &'a self,
        query: PaymentRecordStatisticsQuery,
    ) -> CommercePaymentFuture<'a, PaymentRecordStatistics> {
        self.records.fetch_payment_statistics(query)
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
        Err(message) => return unauthorized_response(None, message),
    };
    let query =
        match PaymentMethodListQuery::new(&subject.tenant_id, subject.organization_id.as_deref()) {
            Ok(query) => query,
            Err(error) => return validation_response(None, error.message()),
        };

    match state.store.list_payment_methods(query).await {
        Ok(items) => {
            let methods = items
                .into_iter()
                .map(map_payment_method)
                .filter(|method| matches_client_type(method, params.client_type.as_deref()))
                .collect::<Vec<_>>();
            success_item(None, methods)
        }
        Err(error) => payment_system_response(None, "payment methods read model is unavailable", error),
    }
}

async fn list_payment_records(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Query(params): Query<PaymentRecordsQueryParams>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(None, message),
    };
    // Phase 1.3：在 handler 解析标准分页参数（page/page_size），下推为 offset/limit 到 SQL。
    // 默认 page=1, page_size=20，page_size 上限 200（PAGINATION_SPEC §2）。
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = match PaymentRecordListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query.with_paging(page_params.offset, page_params.page_size),
        Err(error) => return validation_response(None, error.message()),
    };

    match state.store.list_payment_records(query).await {
        Ok(page) => {
            // Phase 1.3：store 已在 SQL 层完成 LIMIT/OFFSET 并返回真实 total_items，
            // handler 不再做进程内 skip/take（PAGINATION_SPEC §2 合规）。
            let items: Vec<_> = page
                .items
                .into_iter()
                .map(map_payment_record)
                .collect();
            success_list(None, items, page.total_items, page_params)
        }
        Err(error) => payment_system_response(None, "payment records read model is unavailable", error),
    }
}

async fn list_order_payments(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(order_id): Path<String>,
    Query(params): Query<OrderPaymentsQueryParams>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = match PaymentRecordOrderListQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &order_id,
    ) {
        Ok(query) => query.with_paging(page_params.offset, page_params.page_size),
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.list_payment_records_by_order(query).await {
        Ok(page) => {
            let items = page
                .items
                .into_iter()
                .map(map_payment_record)
                .collect::<Vec<_>>();
            success_list(ctx, items, page.total_items, page_params)
        }
        Err(error) => payment_system_response(ctx, "order payment read model is unavailable", error),
    }
}

async fn retrieve_payment_record(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(payment_id): Path<String>,
) -> Response {
    match load_payment_record(state, runtime_context, payment_id).await {
        Ok(record) => {
            success_item(None, map_payment_record(record))
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
        Ok(record) => success_item(None, map_payment_attempt_record(record)),
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
            success_item(None, map_payment_record(record))
        }
        Err(response) => response,
    }
}

async fn retrieve_payment_status_by_out_trade_no(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(out_trade_no): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match PaymentRecordOutTradeNoQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &out_trade_no,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.retrieve_payment_record_by_out_trade_no(query).await {
        Ok(record) => success_item(ctx, map_payment_record(record)),
        Err(error) => payment_system_response(ctx, "payment record read model is unavailable", error),
    }
}

async fn fetch_payment_statistics(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let query = match PaymentRecordStatisticsQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
    ) {
        Ok(query) => query,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.fetch_payment_statistics(query).await {
        Ok(statistics) => success_item(ctx, map_payment_statistics(statistics)),
        Err(error) => {
            payment_system_response(ctx, "payment statistics read model is unavailable", error)
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
        Err(message) => return Err(unauthorized_response(None, message)),
    };
    let query = match PaymentRecordDetailQuery::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_id,
    ) {
        Ok(query) => query,
        Err(error) => return Err(validation_response(None, error.message())),
    };

    state
        .store
        .retrieve_payment_record(query)
        .await
        .map_err(|error| payment_system_response(None, "payment record read model is unavailable", error))
}

async fn reconcile_payment(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    body: Json<ReconcilePaymentRequest>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let _write_headers = match validate_app_write_payload(
        &headers,
        "payments.reconcile",
        &body.0,
        |idempotency_key| format!("payment-reconcile-{idempotency_key}"),
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
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

    let record_result = if reconcile_type.eq_ignore_ascii_case("ORDER_ID") {
        let Some(order_id) = order_id else {
            return validation(ctx, "orderId is required for ORDER_ID reconciliation");
        };
        let query = match PaymentRecordDetailQuery::new(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
            order_id,
        ) {
            Ok(query) => query,
            Err(error) => return validation(ctx, error.message()),
        };
        state.store.retrieve_payment_record(query).await
    } else if out_trade_no.is_some() || reconcile_type.eq_ignore_ascii_case("OUT_TRADE_NO") {
        let Some(out_trade_no) = out_trade_no else {
            return validation(ctx, "outTradeNo is required for OUT_TRADE_NO reconciliation");
        };
        let query = match PaymentRecordOutTradeNoQuery::new(
            &subject.tenant_id,
            subject.organization_id.as_deref(),
            &subject.user_id,
            out_trade_no,
        ) {
            Ok(query) => query,
            Err(error) => return validation(ctx, error.message()),
        };
        state
            .store
            .retrieve_payment_record_by_out_trade_no(query)
            .await
    } else {
        return validation(ctx, "reconcileType must be ORDER_ID or OUT_TRADE_NO");
    };

    match record_result {
        Ok(record) => success_item(ctx, map_payment_record(record)),
        Err(error) if error.code() == "not-found" => {
            not_found(ctx, "payment record was not found")
        }
        Err(error) => payment_system_response(ctx, "payment reconcile read model is unavailable", error),
    }
}

async fn create_payment(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    body: Json<CreatePaymentRequest>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(None, message),
    };
    // C9 修复：所有写操作必须校验 Idempotency-Key 与 Sdkwork-Request-Hash 命令头，
    // 保证客户端重试不会触发非幂等副作用。原代码完全跳过此校验。
    let write_headers = match validate_app_write_payload(
        &headers,
        "payment-create",
        &body.0,
        |idempotency_key| format!("payment-create-{idempotency_key}"),
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
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
        &write_headers.request_no,
        &write_headers.idempotency_key,
    ) {
        Ok(command) => command,
        Err(error) => return validation_response(None, error.message()),
    };

    match state.store.pay_owner_order(command).await {
        Ok(outcome) => {
            success_item(None, map_create_payment(outcome))
        }
        Err(error) => payment_system_response(None, "payment create command failed", error),
    }
}

async fn close_payment_record(
    State(state): State<AppPaymentState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(payment_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized(ctx, message),
    };
    let payload = write_payload_with_route_param("paymentId", &payment_id, &serde_json::json!({}));
    let _write_headers = match validate_app_write_payload(
        &headers,
        "payments.close",
        &payload,
        |idempotency_key| format!("payment-close-{payment_id}-{idempotency_key}"),
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let command = match ClosePaymentRecordCommand::new(
        &subject.tenant_id,
        subject.organization_id.as_deref(),
        &subject.user_id,
        &payment_id,
    ) {
        Ok(command) => command,
        Err(error) => return validation(ctx, error.message()),
    };

    match state.store.close_payment_record(command).await {
        Ok(()) => success_command_accepted(ctx, Some(payment_id)),
        Err(error) => payment_system_response(ctx, "payment close command failed", error),
    }
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
    let product_types = sdkwork_payment_service::wire_product_types_from_scene_codes(&value.scene_codes)
        .into_iter()
        .map(|(code, name)| PaymentMethodProductTypeResponse {
            code,
            name,
            available: true,
        })
        .collect();
    PaymentMethodResponse {
        method_id: value.id,
        code: value.method_key,
        method_name: value.display_name,
        available: true,
        sort: value.sort_order,
        product_types,
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

fn map_payment_statistics(value: PaymentRecordStatistics) -> PaymentStatisticsResponse {
    PaymentStatisticsResponse {
        total_payments: value.total_payments,
        pending_payments: value.pending_payments,
        success_payments: value.success_payments,
        failed_payments: value.failed_payments,
        timeout_payments: value.timeout_payments,
        closed_payments: value.closed_payments,
    }
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

fn payment_system_response(
    context: Option<&WebRequestContext>,
    _label: &str,
    error: CommerceServiceError,
) -> Response {
    map_service_error(context, error)
}

fn unauthorized_response(context: Option<&WebRequestContext>, message: String) -> Response {
    unauthorized(context, message)
}

fn validation_response(context: Option<&WebRequestContext>, message: impl Into<String>) -> Response {
    validation(context, message)
}
