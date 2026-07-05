use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use sdkwork_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use sdkwork_utils_rust::OffsetListPageParams;
use sdkwork_web_core::WebRequestContext;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, sqlite::SqliteRow, PgPool, Row, SqlitePool};

use crate::api_response::{
    conflict, map_service_error, not_found, success_command_accepted, success_item, success_list,
    unauthorized, validation,
};
use crate::command_headers::{
    validate_write_payload, AppWriteCommandHeaders, WriteCommandHeaderError,
};
use crate::subject::backend_runtime_subject_from_extension;

pub type CommerceBackendPaymentAdminFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

/// C15 修复：webhook 单事件最大重放次数。超过即拒绝并返回 409 Conflict。
/// 行业实践（Stripe/Adyen）通常限制 5-10 次，这里取 5 次作为保守上限。
const WEBHOOK_MAX_RETRIES: i64 = 5;

/// C15 修复：webhook 重放结果，用于 handler 区分 404/409/200。
#[derive(Debug, Clone, Serialize)]
pub enum WebhookReplayResult {
    Queued(serde_json::Value),
    NotFound,
    LimitExceeded { current_retries: i64 },
}

pub trait CommerceBackendPaymentAdminStore: Send + Sync {
    fn list_payment_methods<'a>(
        &'a self,
        query: BackendPaymentMethodListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodListPage>;

    fn upsert_payment_method<'a>(
        &'a self,
        command: UpsertBackendPaymentMethodCommand,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodView>;

    fn list_provider_accounts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn upsert_provider_account<'a>(
        &'a self,
        payload: BackendProviderAccountPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn list_channels<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn upsert_channel<'a>(
        &'a self,
        payload: BackendPaymentChannelPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn list_route_rules<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn upsert_route_rule<'a>(
        &'a self,
        payload: BackendRouteRulePayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn delete_route_rule<'a>(
        &'a self,
        scope: BackendTenantScope,
        route_rule_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, ()>;

    fn list_attempts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn list_webhook_events<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, WebhookReplayResult>;

    fn list_reconciliation_runs<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage>;

    fn create_reconciliation_run<'a>(
        &'a self,
        payload: BackendReconciliationRunPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;
}

#[derive(Clone)]
struct BackendPaymentAdminState {
    store: Arc<dyn CommerceBackendPaymentAdminStore>,
}

#[derive(Debug, Clone)]
pub struct BackendTenantScope {
    pub tenant_id: String,
    pub organization_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentMethodListParams {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BackendListQueryParams {
    #[serde(default)]
    page: Option<i64>,
    #[serde(default)]
    page_size: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BackendPaymentMethodListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub status: Option<String>,
    pub offset: i64,
    pub limit: i64,
}

#[derive(Debug, Clone)]
pub struct BackendTenantListQuery {
    pub scope: BackendTenantScope,
    pub offset: i64,
    pub limit: i64,
}

/// Phase 1.3：标准分页结果，store 一次性返回当前页 items + 满足条件的总记录数。
#[derive(Debug, Clone, Serialize)]
pub struct BackendListPage<T> {
    pub items: Vec<T>,
    pub total_items: i64,
}

pub type BackendPaymentMethodListPage = BackendListPage<BackendPaymentMethodView>;
pub type BackendJsonListPage = BackendListPage<serde_json::Value>;

#[derive(Debug, Clone)]
pub struct BackendPaymentMethodView {
    pub id: String,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub method_key: String,
    pub display_name: String,
    pub provider_code: String,
    pub status: String,
    pub sort_order: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct UpsertBackendPaymentMethodCommand {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub method_key: String,
    pub display_name: String,
    pub provider_code: String,
    pub status: String,
    pub sort_order: i64,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpsertPaymentMethodBody {
    method_key: Option<String>,
    display_name: Option<String>,
    provider_code: Option<String>,
    status: Option<String>,
    sort_order: Option<i64>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpsertProviderAccountBody {
    account_no: Option<String>,
    provider_code: Option<String>,
    merchant_id: Option<String>,
    environment: Option<String>,
    country_code: Option<String>,
    settlement_currency: Option<String>,
    secret_ref: Option<String>,
    webhook_secret_ref: Option<String>,
    certificate_ref: Option<String>,
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackendProviderAccountPayload {
    pub id: Option<String>,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub account_no: String,
    pub provider_code: String,
    pub merchant_id: String,
    pub environment: String,
    pub country_code: String,
    pub settlement_currency: String,
    pub secret_ref: String,
    pub webhook_secret_ref: Option<String>,
    pub certificate_ref: Option<String>,
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpsertChannelBody {
    channel_no: Option<String>,
    provider_account_id: Option<String>,
    method_id: Option<String>,
    scene_code: Option<String>,
    currency_code: Option<String>,
    country_code: Option<String>,
    status: Option<String>,
    priority: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BackendPaymentChannelPayload {
    pub id: Option<String>,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub channel_no: String,
    pub provider_account_id: String,
    pub method_id: String,
    pub scene_code: String,
    pub currency_code: String,
    pub country_code: String,
    pub status: String,
    pub priority: i64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct UpsertRouteRuleBody {
    rule_no: Option<String>,
    priority: Option<i64>,
    purchase_type: Option<String>,
    country_code: Option<String>,
    currency_code: Option<String>,
    client_platform: Option<String>,
    amount_min: Option<String>,
    amount_max: Option<String>,
    user_segment: Option<String>,
    risk_level: Option<String>,
    channel_id: Option<String>,
    status: Option<String>,
    starts_at: Option<String>,
    ends_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackendRouteRulePayload {
    pub id: Option<String>,
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub rule_no: String,
    pub priority: i64,
    pub purchase_type: Option<String>,
    pub country_code: Option<String>,
    pub currency_code: Option<String>,
    pub client_platform: Option<String>,
    pub amount_min: Option<String>,
    pub amount_max: Option<String>,
    pub user_segment: Option<String>,
    pub risk_level: Option<String>,
    pub channel_id: String,
    pub status: String,
    pub starts_at: Option<String>,
    pub ends_at: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CreateReconciliationRunBody {
    provider_code: Option<String>,
    account_id: Option<String>,
    provider_account_id: Option<String>,
    statement_date: Option<String>,
    reconciliation_type: Option<String>,
    period_start: Option<String>,
    period_end: Option<String>,
    currency_code: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackendReconciliationRunPayload {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub provider_code: String,
    pub provider_account_id: String,
    pub reconciliation_type: String,
    pub period_start: String,
    pub period_end: String,
    pub currency_code: String,
    pub request_no: String,
    pub idempotency_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BackendPaymentMethodResponse {
    id: String,
    method_key: String,
    display_name: String,
    provider_code: String,
    status: String,
    sort_order: i64,
}

#[derive(Clone)]
struct SqliteBackendPaymentAdminStore {
    pool: SqlitePool,
}

#[derive(Clone)]
struct PostgresBackendPaymentAdminStore {
    pool: PgPool,
}

impl SqliteBackendPaymentAdminStore {
    fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

impl PostgresBackendPaymentAdminStore {
    fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl CommerceBackendPaymentAdminStore for SqliteBackendPaymentAdminStore {
    fn list_payment_methods<'a>(
        &'a self,
        query: BackendPaymentMethodListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodListPage> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, method_key, display_name, provider_code,
                       status, sort_order, created_at, updated_at,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_method
                WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                  AND (? IS NULL OR LOWER(COALESCE(status, '')) = LOWER(CAST(? AS TEXT)))
                ORDER BY sort_order ASC, created_at ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.status.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment methods: {error}"))
            })?;

            let total_items = sqlite_total_count(&rows);
            let items = rows.iter().map(map_method_row_sqlite).collect();

            Ok(BackendPaymentMethodListPage { items, total_items })
        })
    }

    fn upsert_payment_method<'a>(
        &'a self,
        command: UpsertBackendPaymentMethodCommand,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodView> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let id = stable_storage_id(&[
                "payment-method",
                &command.tenant_id,
                command.organization_id.as_deref().unwrap_or("global"),
                &command.method_key,
            ]);

            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_method
                    (id, tenant_id, organization_id, method_key, display_name, provider_code,
                     status, sort_order, request_no, idempotency_key, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (tenant_id, organization_id, method_key) DO UPDATE SET
                    display_name = EXCLUDED.display_name,
                    provider_code = EXCLUDED.provider_code,
                    status = EXCLUDED.status,
                    sort_order = EXCLUDED.sort_order,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, tenant_id, organization_id, method_key, display_name, provider_code,
                          status, sort_order, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.method_key)
            .bind(&command.display_name)
            .bind(&command.provider_code)
            .bind(&command.status)
            .bind(command.sort_order)
            .bind(&command.request_no)
            .bind(&command.idempotency_key)
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to upsert payment method: {error}"))
            })?;

            Ok(map_method_row_sqlite(&row))
        })
    }

    fn list_provider_accounts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, account_no, provider_code, merchant_id, environment, country_code,
                       settlement_currency, status, COUNT(*) OVER() AS total_count
                FROM commerce_payment_provider_account
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list provider accounts: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": sqlite_string(row, "id"),
                        "accountNo": sqlite_string(row, "account_no"),
                        "providerCode": sqlite_string(row, "provider_code"),
                        "merchantId": sqlite_string(row, "merchant_id"),
                        "environment": sqlite_string(row, "environment"),
                        "countryCode": sqlite_string(row, "country_code"),
                        "settlementCurrency": sqlite_string(row, "settlement_currency"),
                        "status": sqlite_string(row, "status"),
                    })
                })
                .collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_provider_account<'a>(
        &'a self,
        payload: BackendProviderAccountPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["provider-account", &payload.tenant_id, &payload.account_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_provider_account
                    (id, tenant_id, organization_id, account_no, provider_code, merchant_id, environment, country_code,
                     settlement_currency, secret_ref, webhook_secret_ref, certificate_ref, status, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (tenant_id, account_no) DO UPDATE SET
                    provider_code = EXCLUDED.provider_code,
                    merchant_id = EXCLUDED.merchant_id,
                    environment = EXCLUDED.environment,
                    country_code = EXCLUDED.country_code,
                    settlement_currency = EXCLUDED.settlement_currency,
                    secret_ref = EXCLUDED.secret_ref,
                    webhook_secret_ref = EXCLUDED.webhook_secret_ref,
                    certificate_ref = EXCLUDED.certificate_ref,
                    status = EXCLUDED.status,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, account_no, provider_code, merchant_id, environment, country_code, settlement_currency, status
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.account_no)
            .bind(&payload.provider_code)
            .bind(&payload.merchant_id)
            .bind(&payload.environment)
            .bind(&payload.country_code)
            .bind(&payload.settlement_currency)
            .bind(&payload.secret_ref)
            .bind(payload.webhook_secret_ref.as_deref())
            .bind(payload.certificate_ref.as_deref())
            .bind(&payload.status)
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert provider account: {error}")))?;
            Ok(serde_json::json!({
                "id": sqlite_string(&row, "id"),
                "accountNo": sqlite_string(&row, "account_no"),
                "providerCode": sqlite_string(&row, "provider_code"),
                "merchantId": sqlite_string(&row, "merchant_id"),
                "environment": sqlite_string(&row, "environment"),
                "countryCode": sqlite_string(&row, "country_code"),
                "settlementCurrency": sqlite_string(&row, "settlement_currency"),
                "status": sqlite_string(&row, "status"),
            }))
        })
    }

    fn list_channels<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, channel_no, provider_account_id, method_id, scene_code, currency_code,
                       country_code, status, priority, COUNT(*) OVER() AS total_count
                FROM commerce_payment_channel
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY priority ASC, created_at ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list channels: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": sqlite_string(row, "id"),
                        "channelNo": sqlite_string(row, "channel_no"),
                        "providerAccountId": sqlite_string(row, "provider_account_id"),
                        "methodId": sqlite_string(row, "method_id"),
                        "sceneCode": sqlite_string(row, "scene_code"),
                        "currencyCode": sqlite_string(row, "currency_code"),
                        "countryCode": sqlite_string(row, "country_code"),
                        "status": sqlite_string(row, "status"),
                        "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
                    })
                })
                .collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_channel<'a>(
        &'a self,
        payload: BackendPaymentChannelPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["payment-channel", &payload.tenant_id, &payload.channel_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_channel
                    (id, tenant_id, organization_id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (tenant_id, channel_no) DO UPDATE SET
                    provider_account_id = EXCLUDED.provider_account_id,
                    method_id = EXCLUDED.method_id,
                    scene_code = EXCLUDED.scene_code,
                    currency_code = EXCLUDED.currency_code,
                    country_code = EXCLUDED.country_code,
                    status = EXCLUDED.status,
                    priority = EXCLUDED.priority,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.channel_no)
            .bind(&payload.provider_account_id)
            .bind(&payload.method_id)
            .bind(&payload.scene_code)
            .bind(&payload.currency_code)
            .bind(&payload.country_code)
            .bind(&payload.status)
            .bind(payload.priority)
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert channel: {error}")))?;
            Ok(serde_json::json!({
                "id": sqlite_string(&row, "id"),
                "channelNo": sqlite_string(&row, "channel_no"),
                "providerAccountId": sqlite_string(&row, "provider_account_id"),
                "methodId": sqlite_string(&row, "method_id"),
                "sceneCode": sqlite_string(&row, "scene_code"),
                "currencyCode": sqlite_string(&row, "currency_code"),
                "countryCode": sqlite_string(&row, "country_code"),
                "status": sqlite_string(&row, "status"),
                "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
            }))
        })
    }

    fn list_route_rules<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, rule_no, priority, purchase_type, country_code, currency_code, client_platform,
                       amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at,
                       ends_at, COUNT(*) OVER() AS total_count
                FROM commerce_payment_route_rule
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY priority ASC, created_at ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list route rules: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows.iter().map(map_route_rule_sqlite).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_route_rule<'a>(
        &'a self,
        payload: BackendRouteRulePayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["payment-route-rule", &payload.tenant_id, &payload.rule_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_route_rule
                    (id, tenant_id, organization_id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT (tenant_id, rule_no) DO UPDATE SET
                    priority = EXCLUDED.priority,
                    purchase_type = EXCLUDED.purchase_type,
                    country_code = EXCLUDED.country_code,
                    currency_code = EXCLUDED.currency_code,
                    client_platform = EXCLUDED.client_platform,
                    amount_min = EXCLUDED.amount_min,
                    amount_max = EXCLUDED.amount_max,
                    user_segment = EXCLUDED.user_segment,
                    risk_level = EXCLUDED.risk_level,
                    channel_id = EXCLUDED.channel_id,
                    status = EXCLUDED.status,
                    starts_at = EXCLUDED.starts_at,
                    ends_at = EXCLUDED.ends_at,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.rule_no)
            .bind(payload.priority)
            .bind(payload.purchase_type.as_deref())
            .bind(payload.country_code.as_deref())
            .bind(payload.currency_code.as_deref())
            .bind(payload.client_platform.as_deref())
            .bind(payload.amount_min.as_deref())
            .bind(payload.amount_max.as_deref())
            .bind(payload.user_segment.as_deref())
            .bind(payload.risk_level.as_deref())
            .bind(&payload.channel_id)
            .bind(&payload.status)
            .bind(payload.starts_at.as_deref())
            .bind(payload.ends_at.as_deref())
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert route rule: {error}")))?;
            Ok(map_route_rule_sqlite(&row))
        })
    }

    fn delete_route_rule<'a>(
        &'a self,
        scope: BackendTenantScope,
        route_rule_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query("DELETE FROM commerce_payment_route_rule WHERE id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))")
                .bind(&route_rule_id)
                .bind(&scope.tenant_id)
                .bind(scope.organization_id.as_deref())
                .bind(scope.organization_id.as_deref())
                .execute(&self.pool)
                .await
                .map_err(|error| {
                    CommerceServiceError::storage(format!("failed to delete route rule: {error}"))
                })?;
            Ok(())
        })
    }

    fn list_attempts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, payment_intent_id, attempt_no, provider_code, channel_id, amount, currency_code,
                       status, provider_transaction_id, created_at, COUNT(*) OVER() AS total_count
                FROM commerce_payment_attempt
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list attempts: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows.iter().map(map_attempt_sqlite).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn list_webhook_events<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, event_id, provider_code, event_type, status, received_at, processed_at, retries,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_webhook_event
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY received_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list webhook events: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows.iter().map(map_webhook_event_sqlite).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, WebhookReplayResult> {
        Box::pin(async move {
            let now = current_timestamp_string();
            // C15 修复：UPDATE 带 retries < MAX 谓词，原子化阻止超限重放。
            let row = sqlx::query(
                "UPDATE commerce_payment_webhook_event SET status = 'queued', processed_at = NULL, retries = COALESCE(retries, 0) + 1, updated_at = ? WHERE event_id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) AND COALESCE(retries, 0) < ? RETURNING id, event_id, provider_code, event_type, status, received_at, processed_at, retries",
            )
            .bind(&now)
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(WEBHOOK_MAX_RETRIES)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to replay webhook event: {error}")))?;

            if let Some(row) = row {
                return Ok(WebhookReplayResult::Queued(map_webhook_event_sqlite(&row)));
            }

            // C15 修复：UPDATE 未命中，需区分 404（事件不存在）与 409（达到重放上限）。
            let existing = sqlx::query(
                "SELECT retries FROM commerce_payment_webhook_event WHERE event_id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))",
            )
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to inspect webhook event: {error}")))?;

            match existing {
                None => Ok(WebhookReplayResult::NotFound),
                Some(row) => Ok(WebhookReplayResult::LimitExceeded {
                    current_retries: row.try_get::<i64, _>("retries").unwrap_or(WEBHOOK_MAX_RETRIES),
                }),
            }
        })
    }

    fn list_reconciliation_runs<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, run_no, provider_code, provider_account_id, reconciliation_type, period_start,
                       period_end, status, matched_count, mismatched_count, currency_code, created_at,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_reconciliation_run
                WHERE tenant_id = CAST(? AS TEXT)
                  AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list reconciliation runs: {error}")))?;
            let total_items = sqlite_total_count(&rows);
            let items = rows.iter().map(map_reconciliation_run_sqlite).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn create_reconciliation_run<'a>(
        &'a self,
        payload: BackendReconciliationRunPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let id = stable_storage_id(&[
                "reconciliation-run",
                &payload.tenant_id,
                &payload.provider_account_id,
                &payload.period_start,
            ]);
            let run_no =
                stable_storage_id(&["recon", &payload.provider_code, &payload.period_start]);
            let row = sqlx::query(
                "INSERT INTO commerce_payment_reconciliation_run (id, tenant_id, organization_id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, unmatched_count, total_difference_amount, currency_code, request_no, idempotency_key, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'queued', 0, 0, 0, '0', ?, ?, ?, ?, ?) RETURNING id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, currency_code, created_at",
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&run_no)
            .bind(&payload.provider_code)
            .bind(&payload.provider_account_id)
            .bind(&payload.reconciliation_type)
            .bind(&payload.period_start)
            .bind(&payload.period_end)
            .bind(&payload.currency_code)
            .bind(&payload.request_no)
            .bind(&payload.idempotency_key)
            .bind(&now)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to create reconciliation run: {error}")))?;
            Ok(map_reconciliation_run_sqlite(&row))
        })
    }
}

impl CommerceBackendPaymentAdminStore for PostgresBackendPaymentAdminStore {
    fn list_payment_methods<'a>(
        &'a self,
        query: BackendPaymentMethodListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodListPage> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, method_key, display_name, provider_code,
                       status, sort_order, created_at, updated_at,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_method
                WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                  AND ($3::text IS NULL OR LOWER(COALESCE(status, '')) = LOWER($3::text))
                ORDER BY sort_order ASC, created_at ASC
                LIMIT $4 OFFSET $5
                "#,
            )
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment methods: {error}"))
            })?;

            let total_items = pg_total_count(&rows);
            let items = rows.iter().map(map_method_row_pg).collect();

            Ok(BackendPaymentMethodListPage { items, total_items })
        })
    }

    fn upsert_payment_method<'a>(
        &'a self,
        command: UpsertBackendPaymentMethodCommand,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodView> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let id = stable_storage_id(&[
                "payment-method",
                &command.tenant_id,
                command.organization_id.as_deref().unwrap_or("global"),
                &command.method_key,
            ]);

            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_method
                    (id, tenant_id, organization_id, method_key, display_name, provider_code,
                     status, sort_order, request_no, idempotency_key, created_at, updated_at)
                VALUES (CAST($1 AS TEXT), CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, $11, $11)
                ON CONFLICT (tenant_id, organization_id, method_key) DO UPDATE SET
                    display_name = EXCLUDED.display_name,
                    provider_code = EXCLUDED.provider_code,
                    status = EXCLUDED.status,
                    sort_order = EXCLUDED.sort_order,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, tenant_id, organization_id, method_key, display_name, provider_code,
                          status, sort_order, created_at, updated_at
                "#,
            )
            .bind(&id)
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.method_key)
            .bind(&command.display_name)
            .bind(&command.provider_code)
            .bind(&command.status)
            .bind(command.sort_order)
            .bind(&command.request_no)
            .bind(&command.idempotency_key)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to upsert payment method: {error}"))
            })?;

            Ok(map_method_row_pg(&row))
        })
    }

    fn list_provider_accounts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, account_no, provider_code, merchant_id, environment, country_code,
                       settlement_currency, status, COUNT(*) OVER() AS total_count
                FROM commerce_payment_provider_account
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list provider accounts: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": pg_string(row, "id"),
                        "accountNo": pg_string(row, "account_no"),
                        "providerCode": pg_string(row, "provider_code"),
                        "merchantId": pg_string(row, "merchant_id"),
                        "environment": pg_string(row, "environment"),
                        "countryCode": pg_string(row, "country_code"),
                        "settlementCurrency": pg_string(row, "settlement_currency"),
                        "status": pg_string(row, "status"),
                    })
                })
                .collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_provider_account<'a>(
        &'a self,
        payload: BackendProviderAccountPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["provider-account", &payload.tenant_id, &payload.account_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_provider_account
                    (id, tenant_id, organization_id, account_no, provider_code, merchant_id, environment, country_code,
                     settlement_currency, secret_ref, webhook_secret_ref, certificate_ref, status, created_at, updated_at)
                VALUES (CAST($1 AS TEXT), CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)
                ON CONFLICT (tenant_id, account_no) DO UPDATE SET
                    provider_code = EXCLUDED.provider_code,
                    merchant_id = EXCLUDED.merchant_id,
                    environment = EXCLUDED.environment,
                    country_code = EXCLUDED.country_code,
                    settlement_currency = EXCLUDED.settlement_currency,
                    secret_ref = EXCLUDED.secret_ref,
                    webhook_secret_ref = EXCLUDED.webhook_secret_ref,
                    certificate_ref = EXCLUDED.certificate_ref,
                    status = EXCLUDED.status,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, account_no, provider_code, merchant_id, environment, country_code, settlement_currency, status
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.account_no)
            .bind(&payload.provider_code)
            .bind(&payload.merchant_id)
            .bind(&payload.environment)
            .bind(&payload.country_code)
            .bind(&payload.settlement_currency)
            .bind(&payload.secret_ref)
            .bind(payload.webhook_secret_ref.as_deref())
            .bind(payload.certificate_ref.as_deref())
            .bind(&payload.status)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert provider account: {error}")))?;
            Ok(serde_json::json!({
                "id": pg_string(&row, "id"),
                "accountNo": pg_string(&row, "account_no"),
                "providerCode": pg_string(&row, "provider_code"),
                "merchantId": pg_string(&row, "merchant_id"),
                "environment": pg_string(&row, "environment"),
                "countryCode": pg_string(&row, "country_code"),
                "settlementCurrency": pg_string(&row, "settlement_currency"),
                "status": pg_string(&row, "status"),
            }))
        })
    }

    fn list_channels<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, channel_no, provider_account_id, method_id, scene_code, currency_code,
                       country_code, status, priority, COUNT(*) OVER() AS total_count
                FROM commerce_payment_channel
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY priority ASC, created_at ASC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list channels: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows
                .iter()
                .map(|row| {
                    serde_json::json!({
                        "id": pg_string(row, "id"),
                        "channelNo": pg_string(row, "channel_no"),
                        "providerAccountId": pg_string(row, "provider_account_id"),
                        "methodId": pg_string(row, "method_id"),
                        "sceneCode": pg_string(row, "scene_code"),
                        "currencyCode": pg_string(row, "currency_code"),
                        "countryCode": pg_string(row, "country_code"),
                        "status": pg_string(row, "status"),
                        "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
                    })
                })
                .collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_channel<'a>(
        &'a self,
        payload: BackendPaymentChannelPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["payment-channel", &payload.tenant_id, &payload.channel_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_channel
                    (id, tenant_id, organization_id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority, created_at, updated_at)
                VALUES (CAST($1 AS TEXT), CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), CAST($5 AS TEXT), CAST($6 AS TEXT), $7, $8, $9, $10, $11, $12, $12)
                ON CONFLICT (tenant_id, channel_no) DO UPDATE SET
                    provider_account_id = EXCLUDED.provider_account_id,
                    method_id = EXCLUDED.method_id,
                    scene_code = EXCLUDED.scene_code,
                    currency_code = EXCLUDED.currency_code,
                    country_code = EXCLUDED.country_code,
                    status = EXCLUDED.status,
                    priority = EXCLUDED.priority,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.channel_no)
            .bind(&payload.provider_account_id)
            .bind(&payload.method_id)
            .bind(&payload.scene_code)
            .bind(&payload.currency_code)
            .bind(&payload.country_code)
            .bind(&payload.status)
            .bind(payload.priority)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert channel: {error}")))?;
            Ok(serde_json::json!({
                "id": pg_string(&row, "id"),
                "channelNo": pg_string(&row, "channel_no"),
                "providerAccountId": pg_string(&row, "provider_account_id"),
                "methodId": pg_string(&row, "method_id"),
                "sceneCode": pg_string(&row, "scene_code"),
                "currencyCode": pg_string(&row, "currency_code"),
                "countryCode": pg_string(&row, "country_code"),
                "status": pg_string(&row, "status"),
                "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
            }))
        })
    }

    fn list_route_rules<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, rule_no, priority, purchase_type, country_code, currency_code, client_platform,
                       amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at,
                       ends_at, COUNT(*) OVER() AS total_count
                FROM commerce_payment_route_rule
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY priority ASC, created_at ASC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list route rules: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows.iter().map(map_route_rule_pg).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn upsert_route_rule<'a>(
        &'a self,
        payload: BackendRouteRulePayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let id = payload.id.clone().unwrap_or_else(|| {
                stable_storage_id(&["payment-route-rule", &payload.tenant_id, &payload.rule_no])
            });
            let now = current_timestamp_string();
            let row = sqlx::query(
                r#"
                INSERT INTO commerce_payment_route_rule
                    (id, tenant_id, organization_id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at, created_at, updated_at)
                VALUES (CAST($1 AS TEXT), CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, $11, $12, $13, CAST($14 AS TEXT), $15, $16, $17, $18, $18)
                ON CONFLICT (tenant_id, rule_no) DO UPDATE SET
                    priority = EXCLUDED.priority,
                    purchase_type = EXCLUDED.purchase_type,
                    country_code = EXCLUDED.country_code,
                    currency_code = EXCLUDED.currency_code,
                    client_platform = EXCLUDED.client_platform,
                    amount_min = EXCLUDED.amount_min,
                    amount_max = EXCLUDED.amount_max,
                    user_segment = EXCLUDED.user_segment,
                    risk_level = EXCLUDED.risk_level,
                    channel_id = EXCLUDED.channel_id,
                    status = EXCLUDED.status,
                    starts_at = EXCLUDED.starts_at,
                    ends_at = EXCLUDED.ends_at,
                    updated_at = EXCLUDED.updated_at
                RETURNING id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at
                "#,
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&payload.rule_no)
            .bind(payload.priority)
            .bind(payload.purchase_type.as_deref())
            .bind(payload.country_code.as_deref())
            .bind(payload.currency_code.as_deref())
            .bind(payload.client_platform.as_deref())
            .bind(payload.amount_min.as_deref())
            .bind(payload.amount_max.as_deref())
            .bind(payload.user_segment.as_deref())
            .bind(payload.risk_level.as_deref())
            .bind(&payload.channel_id)
            .bind(&payload.status)
            .bind(payload.starts_at.as_deref())
            .bind(payload.ends_at.as_deref())
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to upsert route rule: {error}")))?;
            Ok(map_route_rule_pg(&row))
        })
    }

    fn delete_route_rule<'a>(
        &'a self,
        scope: BackendTenantScope,
        route_rule_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, ()> {
        Box::pin(async move {
            sqlx::query(
                "DELETE FROM commerce_payment_route_rule WHERE id = CAST($1 AS TEXT) AND tenant_id = CAST($2 AS TEXT) AND (organization_id IS NULL AND $3::text IS NULL OR organization_id = CAST($3 AS TEXT))",
            )
                .bind(&route_rule_id)
                .bind(&scope.tenant_id)
                .bind(scope.organization_id.as_deref())
                .execute(&self.pool)
                .await
                .map_err(|error| {
                    CommerceServiceError::storage(format!("failed to delete route rule: {error}"))
                })?;
            Ok(())
        })
    }

    fn list_attempts<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, payment_intent_id, attempt_no, provider_code, channel_id, amount, currency_code,
                       status, provider_transaction_id, created_at, COUNT(*) OVER() AS total_count
                FROM commerce_payment_attempt
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list attempts: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows.iter().map(map_attempt_pg).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn list_webhook_events<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, event_id, provider_code, event_type, status, received_at, processed_at, retries,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_webhook_event
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY received_at DESC, id DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list webhook events: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows.iter().map(map_webhook_event_pg).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, WebhookReplayResult> {
        Box::pin(async move {
            let now = current_timestamp_string();
            // C15 修复：UPDATE 带 retries < MAX 谓词，原子化阻止超限重放。
            let row = sqlx::query(
                "UPDATE commerce_payment_webhook_event SET status = 'queued', processed_at = NULL, retries = COALESCE(retries, 0) + 1, updated_at = $1 WHERE event_id = CAST($2 AS TEXT) AND tenant_id = CAST($3 AS TEXT) AND (organization_id IS NULL AND $4::text IS NULL OR organization_id = CAST($4 AS TEXT)) AND COALESCE(retries, 0) < $5 RETURNING id, event_id, provider_code, event_type, status, received_at, processed_at, retries",
            )
            .bind(&now)
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(WEBHOOK_MAX_RETRIES)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to replay webhook event: {error}")))?;

            if let Some(row) = row {
                return Ok(WebhookReplayResult::Queued(map_webhook_event_pg(&row)));
            }

            // C15 修复：UPDATE 未命中，需区分 404（事件不存在）与 409（达到重放上限）。
            let existing = sqlx::query(
                "SELECT retries FROM commerce_payment_webhook_event WHERE event_id = CAST($1 AS TEXT) AND tenant_id = CAST($2 AS TEXT) AND (organization_id IS NULL AND $3::text IS NULL OR organization_id = CAST($3 AS TEXT))",
            )
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to inspect webhook event: {error}")))?;

            match existing {
                None => Ok(WebhookReplayResult::NotFound),
                Some(row) => Ok(WebhookReplayResult::LimitExceeded {
                    current_retries: row.try_get::<i64, _>("retries").unwrap_or(WEBHOOK_MAX_RETRIES),
                }),
            }
        })
    }

    fn list_reconciliation_runs<'a>(
        &'a self,
        query: BackendTenantListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendJsonListPage> {
        Box::pin(async move {
            let scope = query.scope;
            let rows = sqlx::query(
                r#"
                SELECT id, run_no, provider_code, provider_account_id, reconciliation_type, period_start,
                       period_end, status, matched_count, mismatched_count, currency_code, created_at,
                       COUNT(*) OVER() AS total_count
                FROM commerce_payment_reconciliation_run
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                ORDER BY created_at DESC, id DESC
                LIMIT $3 OFFSET $4
                "#,
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list reconciliation runs: {error}")))?;
            let total_items = pg_total_count(&rows);
            let items = rows.iter().map(map_reconciliation_run_pg).collect();
            Ok(BackendJsonListPage { items, total_items })
        })
    }

    fn create_reconciliation_run<'a>(
        &'a self,
        payload: BackendReconciliationRunPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let id = stable_storage_id(&[
                "reconciliation-run",
                &payload.tenant_id,
                &payload.provider_account_id,
                &payload.period_start,
            ]);
            let run_no =
                stable_storage_id(&["recon", &payload.provider_code, &payload.period_start]);
            let row = sqlx::query(
                "INSERT INTO commerce_payment_reconciliation_run (id, tenant_id, organization_id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, unmatched_count, total_difference_amount, currency_code, request_no, idempotency_key, created_at, updated_at) VALUES (CAST($1 AS TEXT), CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, CAST($6 AS TEXT), $7, $8, $9, 'queued', 0, 0, 0, '0', $10, $11, $12, $13, $13) RETURNING id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, currency_code, created_at",
            )
            .bind(&id)
            .bind(&payload.tenant_id)
            .bind(payload.organization_id.as_deref())
            .bind(&run_no)
            .bind(&payload.provider_code)
            .bind(&payload.provider_account_id)
            .bind(&payload.reconciliation_type)
            .bind(&payload.period_start)
            .bind(&payload.period_end)
            .bind(&payload.currency_code)
            .bind(&payload.request_no)
            .bind(&payload.idempotency_key)
            .bind(&now)
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to create reconciliation run: {error}")))?;
            Ok(map_reconciliation_run_pg(&row))
        })
    }
}

pub fn backend_payment_admin_router_with_sqlite_pool(pool: SqlitePool) -> Router {
    build_backend_payment_admin_router(Arc::new(SqliteBackendPaymentAdminStore::new(pool)))
}

pub fn backend_payment_admin_router_with_postgres_pool(pool: PgPool) -> Router {
    build_backend_payment_admin_router(Arc::new(PostgresBackendPaymentAdminStore::new(pool)))
}

pub fn build_backend_payment_admin_router(
    store: Arc<dyn CommerceBackendPaymentAdminStore>,
) -> Router {
    Router::new()
            .route(
                "/backend/v3/api/payments/methods",
                get(list_methods).post(create_method),
            )
            .route(
                "/backend/v3/api/payments/methods/{methodKey}",
                patch(update_method),
            )
            .route(
                "/backend/v3/api/payments/provider_accounts",
                get(list_provider_accounts).post(create_provider_account),
            )
            .route(
                "/backend/v3/api/payments/provider_accounts/{providerAccountId}",
                patch(update_provider_account),
            )
            .route(
                "/backend/v3/api/payments/channels",
                get(list_channels).post(create_channel),
            )
            .route(
                "/backend/v3/api/payments/route_rules",
                get(list_route_rules).post(create_route_rule),
            )
            .route(
                "/backend/v3/api/payments/route_rules/{routeRuleId}",
                patch(update_route_rule).delete(delete_route_rule),
            )
            .route("/backend/v3/api/payments/attempts", get(list_attempts))
            .route(
                "/backend/v3/api/payments/webhook_events",
                get(list_webhook_events),
            )
            .route(
                "/backend/v3/api/payments/webhook_events/{eventId}/replays",
                post(replay_webhook_event),
            )
            .route(
                "/backend/v3/api/payments/reconciliation_runs",
                get(list_reconciliation_runs).post(create_reconciliation_run),
            )
            .with_state(BackendPaymentAdminState { store })
}

async fn list_methods(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendPaymentMethodListParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendPaymentMethodListQuery {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        status: params.status,
        offset: page_params.offset,
        limit: page_params.page_size,
    };

    match state.store.list_payment_methods(query).await {
        Ok(page) => {
            let items: Vec<_> = page.items.into_iter().map(map_method).collect();
            success_list(ctx, items, page.total_items, page_params)
        }
        Err(error) => backend_payment_error_response(ctx, "payment method list is unavailable", error),
    }
}

async fn create_method(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertPaymentMethodBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let write_headers =
        match validate_backend_write_payload(ctx, &headers, "payment-method-upsert", &body, "pm") {
            Ok(headers) => headers,
            Err(response) => return response,
        };
    let method_key = match require_trimmed_string(ctx, body.method_key, "methodKey") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let display_name = match require_trimmed_string(ctx, body.display_name, "displayName") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let provider_code = match require_trimmed_string(ctx, body.provider_code, "providerCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let status = body.status.unwrap_or_else(|| "active".to_owned());
    let sort_order = body.sort_order.unwrap_or(0);

    let command = UpsertBackendPaymentMethodCommand {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        method_key,
        display_name,
        provider_code,
        status,
        sort_order,
        request_no: write_headers.request_no,
        idempotency_key: write_headers.idempotency_key,
    };

    match state.store.upsert_payment_method(command).await {
        Ok(view) => success_item(ctx, map_method(view)),
        Err(error) => backend_payment_error_response(ctx, "payment method upsert failed", error),
    }
}

async fn update_method(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(method_key): Path<String>,
    Json(body): Json<UpsertPaymentMethodBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let write_headers =
        match validate_backend_write_payload(ctx, &headers, "payment-method-upsert", &body, "pm") {
            Ok(headers) => headers,
            Err(response) => return response,
        };
    let display_name = match require_trimmed_string(ctx, body.display_name, "displayName") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let provider_code = match require_trimmed_string(ctx, body.provider_code, "providerCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let command = UpsertBackendPaymentMethodCommand {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        method_key,
        display_name,
        provider_code,
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        sort_order: body.sort_order.unwrap_or(0),
        request_no: write_headers.request_no,
        idempotency_key: write_headers.idempotency_key,
    };

    match state.store.upsert_payment_method(command).await {
        Ok(view) => success_item(ctx, map_method(view)),
        Err(error) => backend_payment_error_response(ctx, "payment method upsert failed", error),
    }
}

async fn list_provider_accounts(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_provider_accounts(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => {
            backend_payment_error_response(ctx, "payment provider account list is unavailable", error)
        }
    }
}

async fn create_provider_account(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertProviderAccountBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    upsert_provider_account_inner(state, runtime_context, ctx, headers, None, body).await
}

async fn update_provider_account(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(provider_account_id): Path<String>,
    Json(body): Json<UpsertProviderAccountBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    upsert_provider_account_inner(
        state,
        runtime_context,
        ctx,
        headers,
        Some(provider_account_id),
        body,
    )
    .await
}

async fn upsert_provider_account_inner(
    state: BackendPaymentAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    ctx: Option<&WebRequestContext>,
    headers: HeaderMap,
    provider_account_id: Option<String>,
    body: UpsertProviderAccountBody,
) -> Response {
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let _write_headers = match validate_backend_write_payload(
        ctx,
        &headers,
        "payment-provider-account-upsert",
        &body,
        "provider-account",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let account_no = match body.account_no {
        Some(value) => match require_trimmed_string(ctx, Some(value), "accountNo") {
            Ok(value) => value,
            Err(response) => return response,
        },
        None => match provider_account_id.as_ref() {
            Some(value) => value.clone(),
            None => return validation(ctx, "accountNo is required"),
        },
    };
    let provider_code = match require_trimmed_string(ctx, body.provider_code, "providerCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let merchant_id = match require_trimmed_string(ctx, body.merchant_id, "merchantId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let environment = match require_trimmed_string(ctx, body.environment, "environment") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let country_code = match require_trimmed_string(ctx, body.country_code, "countryCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let settlement_currency =
        match require_trimmed_string(ctx, body.settlement_currency, "settlementCurrency") {
            Ok(value) => value,
            Err(response) => return response,
        };
    let secret_ref = match require_trimmed_string(ctx, body.secret_ref, "secretRef") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let payload = BackendProviderAccountPayload {
        id: provider_account_id,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        account_no,
        provider_code,
        merchant_id,
        environment,
        country_code,
        settlement_currency,
        secret_ref,
        webhook_secret_ref: body.webhook_secret_ref,
        certificate_ref: body.certificate_ref,
        status: body.status.unwrap_or_else(|| "active".to_owned()),
    };
    match state.store.upsert_provider_account(payload).await {
        Ok(item) => success_item(ctx, item),
        Err(error) => {
            backend_payment_error_response(ctx, "payment provider account upsert failed", error)
        }
    }
}

async fn list_channels(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_channels(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => backend_payment_error_response(ctx, "payment channel list is unavailable", error),
    }
}

async fn create_channel(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertChannelBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let _write_headers = match validate_backend_write_payload(
        ctx,
        &headers,
        "payment-channel-upsert",
        &body,
        "payment-channel",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let channel_no = match require_trimmed_string(ctx, body.channel_no, "channelNo") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let provider_account_id =
        match require_trimmed_string(ctx, body.provider_account_id, "providerAccountId") {
            Ok(value) => value,
            Err(response) => return response,
        };
    let method_id = match require_trimmed_string(ctx, body.method_id, "methodId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let scene_code = match require_trimmed_string(ctx, body.scene_code, "sceneCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let currency_code = match require_trimmed_string(ctx, body.currency_code, "currencyCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let country_code = match require_trimmed_string(ctx, body.country_code, "countryCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let payload = BackendPaymentChannelPayload {
        id: None,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        channel_no,
        provider_account_id,
        method_id,
        scene_code,
        currency_code,
        country_code,
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        priority: body.priority.unwrap_or(0),
    };
    match state.store.upsert_channel(payload).await {
        Ok(item) => success_item(ctx, item),
        Err(error) => backend_payment_error_response(ctx, "payment channel upsert failed", error),
    }
}

async fn list_route_rules(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_route_rules(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => {
            backend_payment_error_response(ctx, "payment route rule list is unavailable", error)
        }
    }
}

async fn create_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertRouteRuleBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    upsert_route_rule_inner(state, runtime_context, ctx, headers, None, body).await
}

async fn update_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Path(route_rule_id): Path<String>,
    Json(body): Json<UpsertRouteRuleBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    upsert_route_rule_inner(state, runtime_context, ctx, headers, Some(route_rule_id), body).await
}

async fn upsert_route_rule_inner(
    state: BackendPaymentAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    ctx: Option<&WebRequestContext>,
    headers: HeaderMap,
    route_rule_id: Option<String>,
    body: UpsertRouteRuleBody,
) -> Response {
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let _write_headers = match validate_backend_write_payload(
        ctx,
        &headers,
        "payment-route-rule-upsert",
        &body,
        "payment-route-rule",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let rule_no = match body.rule_no {
        Some(value) => match require_trimmed_string(ctx, Some(value), "ruleNo") {
            Ok(value) => value,
            Err(response) => return response,
        },
        None => match route_rule_id.clone() {
            Some(value) => value,
            None => return validation(ctx, "ruleNo is required"),
        },
    };
    let channel_id = match require_trimmed_string(ctx, body.channel_id, "channelId") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let payload = BackendRouteRulePayload {
        id: route_rule_id,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        rule_no,
        priority: body.priority.unwrap_or(0),
        purchase_type: body.purchase_type,
        country_code: body.country_code,
        currency_code: body.currency_code,
        client_platform: body.client_platform,
        amount_min: body.amount_min,
        amount_max: body.amount_max,
        user_segment: body.user_segment,
        risk_level: body.risk_level,
        channel_id,
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        starts_at: body.starts_at,
        ends_at: body.ends_at,
    };
    match state.store.upsert_route_rule(payload).await {
        Ok(item) => success_item(ctx, item),
        Err(error) => backend_payment_error_response(ctx, "payment route rule upsert failed", error),
    }
}

async fn delete_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(route_rule_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.delete_route_rule(scope, route_rule_id).await {
        Ok(()) => success_command_accepted(ctx, None),
        Err(error) => backend_payment_error_response(ctx, "payment route rule delete failed", error),
    }
}

async fn list_attempts(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_attempts(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => backend_payment_error_response(ctx, "payment attempt list is unavailable", error),
    }
}

async fn list_webhook_events(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_webhook_events(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => {
            backend_payment_error_response(ctx, "payment webhook event list is unavailable", error)
        }
    }
}

async fn replay_webhook_event(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    Path(event_id): Path<String>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.replay_webhook_event(scope, event_id).await {
        Ok(WebhookReplayResult::Queued(item)) => success_item(ctx, item),
        Ok(WebhookReplayResult::NotFound) => {
            not_found(ctx, "payment webhook event was not found")
        }
        Ok(WebhookReplayResult::LimitExceeded { current_retries }) => conflict(
            ctx,
            format!(
                "webhook event has reached the replay limit ({WEBHOOK_MAX_RETRIES}); current retries = {current_retries}"
            ),
        ),
        Err(error) => backend_payment_error_response(ctx, "payment webhook replay failed", error),
    }
}

async fn list_reconciliation_runs(
    State(state): State<BackendPaymentAdminState>,
    Query(params): Query<BackendListQueryParams>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let page_params = OffsetListPageParams::parse(params.page, params.page_size);
    let query = BackendTenantListQuery {
        scope: BackendTenantScope {
            tenant_id: subject.tenant_id,
            organization_id: subject.organization_id,
        },
        offset: page_params.offset,
        limit: page_params.page_size,
    };
    match state.store.list_reconciliation_runs(query).await {
        Ok(page) => success_list(ctx, page.items, page.total_items, page_params),
        Err(error) => {
            backend_payment_error_response(ctx, "payment reconciliation run list is unavailable", error)
        }
    }
}

async fn create_reconciliation_run(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    request_context: Option<Extension<WebRequestContext>>,
    headers: HeaderMap,
    Json(body): Json<CreateReconciliationRunBody>,
) -> Response {
    let ctx = request_context.as_ref().map(|Extension(value)| value);
    let subject = match backend_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(ctx, message),
    };
    let write_headers = match validate_backend_write_payload(
        ctx,
        &headers,
        "payment-reconciliation-run-create",
        &body,
        "recon",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let provider_code = match require_trimmed_string(ctx, body.provider_code, "providerCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let provider_account_id = match body
        .provider_account_id
        .or(body.account_id)
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        }) {
        Some(value) => value,
        None => return validation(ctx, "providerAccountId is required"),
    };
    let reconciliation_type =
        match require_trimmed_string(ctx, body.reconciliation_type, "reconciliationType") {
            Ok(value) => value,
            Err(response) => return response,
        };
    let period_start = match body
        .period_start
        .or(body.statement_date)
        .and_then(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_owned())
            }
        }) {
        Some(value) => value,
        None => return validation(ctx, "periodStart is required"),
    };
    let period_end = match body.period_end {
        Some(value) => match require_trimmed_string(ctx, Some(value), "periodEnd") {
            Ok(value) => value,
            Err(response) => return response,
        },
        None => period_start.clone(),
    };
    let currency_code = match require_trimmed_string(ctx, body.currency_code, "currencyCode") {
        Ok(value) => value,
        Err(response) => return response,
    };
    let payload = BackendReconciliationRunPayload {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        provider_code,
        provider_account_id,
        reconciliation_type,
        period_start,
        period_end,
        currency_code,
        request_no: write_headers.request_no,
        idempotency_key: write_headers.idempotency_key,
    };
    match state.store.create_reconciliation_run(payload).await {
        Ok(item) => success_item(ctx, item),
        Err(error) => {
            backend_payment_error_response(ctx, "payment reconciliation run create failed", error)
        }
    }
}

fn map_method(value: BackendPaymentMethodView) -> BackendPaymentMethodResponse {
    BackendPaymentMethodResponse {
        id: value.id,
        method_key: value.method_key,
        display_name: value.display_name,
        provider_code: value.provider_code,
        status: value.status,
        sort_order: value.sort_order,
    }
}

fn backend_payment_error_response(
    ctx: Option<&WebRequestContext>,
    _context: &str,
    error: CommerceServiceError,
) -> Response {
    map_service_error(ctx, error)
}

fn unauthorized_response(
    ctx: Option<&WebRequestContext>,
    message: impl Into<String>,
) -> Response {
    unauthorized(ctx, message)
}

fn map_method_row_sqlite(row: &SqliteRow) -> BackendPaymentMethodView {
    BackendPaymentMethodView {
        id: sqlite_string(row, "id"),
        tenant_id: sqlite_string(row, "tenant_id"),
        organization_id: sqlite_optional_string(row, "organization_id"),
        method_key: sqlite_string(row, "method_key"),
        display_name: sqlite_string(row, "display_name"),
        provider_code: sqlite_string(row, "provider_code"),
        status: sqlite_string(row, "status"),
        sort_order: row.try_get::<i64, _>("sort_order").unwrap_or(0),
        created_at: sqlite_string(row, "created_at"),
        updated_at: sqlite_string(row, "updated_at"),
    }
}

fn map_method_row_pg(row: &PgRow) -> BackendPaymentMethodView {
    BackendPaymentMethodView {
        id: pg_string(row, "id"),
        tenant_id: pg_string(row, "tenant_id"),
        organization_id: pg_optional_string(row, "organization_id"),
        method_key: pg_string(row, "method_key"),
        display_name: pg_string(row, "display_name"),
        provider_code: pg_string(row, "provider_code"),
        status: pg_string(row, "status"),
        sort_order: row.try_get::<i64, _>("sort_order").unwrap_or(0),
        created_at: pg_string(row, "created_at"),
        updated_at: pg_string(row, "updated_at"),
    }
}

fn map_route_rule_sqlite(row: &SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": sqlite_string(row, "id"),
        "ruleNo": sqlite_string(row, "rule_no"),
        "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
        "purchaseType": sqlite_optional_string(row, "purchase_type"),
        "countryCode": sqlite_optional_string(row, "country_code"),
        "currencyCode": sqlite_optional_string(row, "currency_code"),
        "clientPlatform": sqlite_optional_string(row, "client_platform"),
        "amountMin": sqlite_optional_string(row, "amount_min"),
        "amountMax": sqlite_optional_string(row, "amount_max"),
        "userSegment": sqlite_optional_string(row, "user_segment"),
        "riskLevel": sqlite_optional_string(row, "risk_level"),
        "channelId": sqlite_string(row, "channel_id"),
        "status": sqlite_string(row, "status"),
        "startsAt": sqlite_optional_string(row, "starts_at"),
        "endsAt": sqlite_optional_string(row, "ends_at"),
    })
}

fn map_route_rule_pg(row: &PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": pg_string(row, "id"),
        "ruleNo": pg_string(row, "rule_no"),
        "priority": row.try_get::<i64,_>("priority").unwrap_or(0),
        "purchaseType": pg_optional_string(row, "purchase_type"),
        "countryCode": pg_optional_string(row, "country_code"),
        "currencyCode": pg_optional_string(row, "currency_code"),
        "clientPlatform": pg_optional_string(row, "client_platform"),
        "amountMin": pg_optional_string(row, "amount_min"),
        "amountMax": pg_optional_string(row, "amount_max"),
        "userSegment": pg_optional_string(row, "user_segment"),
        "riskLevel": pg_optional_string(row, "risk_level"),
        "channelId": pg_string(row, "channel_id"),
        "status": pg_string(row, "status"),
        "startsAt": pg_optional_string(row, "starts_at"),
        "endsAt": pg_optional_string(row, "ends_at"),
    })
}

fn map_attempt_sqlite(row: &SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": sqlite_string(row, "id"),
        "paymentIntentId": sqlite_string(row, "payment_intent_id"),
        "attemptNo": sqlite_string(row, "attempt_no"),
        "providerCode": sqlite_string(row, "provider_code"),
        "channelId": sqlite_string(row, "channel_id"),
        "amount": sqlite_string(row, "amount"),
        "currencyCode": sqlite_string(row, "currency_code"),
        "status": sqlite_string(row, "status"),
        "providerTransactionId": sqlite_optional_string(row, "provider_transaction_id"),
        "createdAt": sqlite_string(row, "created_at"),
    })
}

fn map_attempt_pg(row: &PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": pg_string(row, "id"),
        "paymentIntentId": pg_string(row, "payment_intent_id"),
        "attemptNo": pg_string(row, "attempt_no"),
        "providerCode": pg_string(row, "provider_code"),
        "channelId": pg_string(row, "channel_id"),
        "amount": pg_string(row, "amount"),
        "currencyCode": pg_string(row, "currency_code"),
        "status": pg_string(row, "status"),
        "providerTransactionId": pg_optional_string(row, "provider_transaction_id"),
        "createdAt": pg_string(row, "created_at"),
    })
}

fn map_webhook_event_sqlite(row: &SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": sqlite_string(row, "id"),
        "eventId": sqlite_string(row, "event_id"),
        "providerCode": sqlite_string(row, "provider_code"),
        "eventType": sqlite_string(row, "event_type"),
        "status": sqlite_string(row, "status"),
        "receivedAt": sqlite_string(row, "received_at"),
        "processedAt": sqlite_optional_string(row, "processed_at"),
        "retries": row.try_get::<i64,_>("retries").unwrap_or(0),
    })
}

fn map_webhook_event_pg(row: &PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": pg_string(row, "id"),
        "eventId": pg_string(row, "event_id"),
        "providerCode": pg_string(row, "provider_code"),
        "eventType": pg_string(row, "event_type"),
        "status": pg_string(row, "status"),
        "receivedAt": pg_string(row, "received_at"),
        "processedAt": pg_optional_string(row, "processed_at"),
        "retries": row.try_get::<i64,_>("retries").unwrap_or(0),
    })
}

fn map_reconciliation_run_sqlite(row: &SqliteRow) -> serde_json::Value {
    serde_json::json!({
        "id": sqlite_string(row, "id"),
        "runNo": sqlite_string(row, "run_no"),
        "providerCode": sqlite_string(row, "provider_code"),
        "providerAccountId": sqlite_string(row, "provider_account_id"),
        "reconciliationType": sqlite_string(row, "reconciliation_type"),
        "periodStart": sqlite_string(row, "period_start"),
        "periodEnd": sqlite_string(row, "period_end"),
        "status": sqlite_string(row, "status"),
        "matchedCount": row.try_get::<i64,_>("matched_count").unwrap_or(0),
        "mismatchedCount": row.try_get::<i64,_>("mismatched_count").unwrap_or(0),
        "currencyCode": sqlite_string(row, "currency_code"),
        "createdAt": sqlite_string(row, "created_at"),
    })
}

fn map_reconciliation_run_pg(row: &PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": pg_string(row, "id"),
        "runNo": pg_string(row, "run_no"),
        "providerCode": pg_string(row, "provider_code"),
        "providerAccountId": pg_string(row, "provider_account_id"),
        "reconciliationType": pg_string(row, "reconciliation_type"),
        "periodStart": pg_string(row, "period_start"),
        "periodEnd": pg_string(row, "period_end"),
        "status": pg_string(row, "status"),
        "matchedCount": row.try_get::<i64,_>("matched_count").unwrap_or(0),
        "mismatchedCount": row.try_get::<i64,_>("mismatched_count").unwrap_or(0),
        "currencyCode": pg_string(row, "currency_code"),
        "createdAt": pg_string(row, "created_at"),
    })
}

fn sqlite_optional_string(row: &SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn sqlite_total_count(rows: &[SqliteRow]) -> i64 {
    rows.first()
        .and_then(|row| row.try_get::<i64, _>("total_count").ok())
        .unwrap_or(0)
}

fn pg_total_count(rows: &[PgRow]) -> i64 {
    rows.first()
        .and_then(|row| row.try_get::<i64, _>("total_count").ok())
        .unwrap_or(0)
}

fn require_trimmed_string(
    ctx: Option<&WebRequestContext>,
    value: Option<String>,
    field: &str,
) -> Result<String, Response> {
    match value {
        Some(value) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        _ => Err(validation(ctx, format!("{field} is required"))),
    }
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

#[allow(clippy::result_large_err)]
fn validate_backend_write_payload(
    ctx: Option<&WebRequestContext>,
    headers: &HeaderMap,
    scope: &str,
    body: &impl Serialize,
    request_no_prefix: &str,
) -> Result<AppWriteCommandHeaders, Response> {
    validate_write_payload(headers, scope, body, |idempotency_key| {
        format!("{request_no_prefix}-{idempotency_key}")
    })
    .map_err(|error| backend_write_header_error(ctx, error))
}

fn backend_write_header_error(
    ctx: Option<&WebRequestContext>,
    error: WriteCommandHeaderError,
) -> Response {
    let message = match error {
        WriteCommandHeaderError::MissingHeader(name) => format!("{name} header is required"),
        WriteCommandHeaderError::InvalidHeader(message) => message.to_owned(),
    };
    validation(ctx, message)
}

fn current_timestamp_string() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn stable_storage_id(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|part| {
            part.chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                        character
                    } else {
                        '-'
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("-")
}
