use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use sdkwork_commerce_contract_service::CommerceServiceError;
use sdkwork_iam_context_service::IamAppContext;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgRow, sqlite::SqliteRow, PgPool, Row, SqlitePool};

use crate::command_headers::{
    validate_write_payload, AppWriteCommandHeaders, WriteCommandHeaderError,
};
use crate::subject::app_runtime_subject_from_extension;

pub type CommerceBackendPaymentAdminFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, CommerceServiceError>> + Send + 'a>>;

pub trait CommerceBackendPaymentAdminStore: Send + Sync {
    fn list_payment_methods<'a>(
        &'a self,
        query: BackendPaymentMethodListQuery,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<BackendPaymentMethodView>>;

    fn upsert_payment_method<'a>(
        &'a self,
        command: UpsertBackendPaymentMethodCommand,
    ) -> CommerceBackendPaymentAdminFuture<'a, BackendPaymentMethodView>;

    fn list_provider_accounts<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

    fn upsert_provider_account<'a>(
        &'a self,
        payload: BackendProviderAccountPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn list_channels<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

    fn upsert_channel<'a>(
        &'a self,
        payload: BackendPaymentChannelPayload,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn list_route_rules<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

    fn list_webhook_events<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value>;

    fn list_reconciliation_runs<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>>;

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
struct BackendPaymentMethodListParams {
    status: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackendPaymentMethodListQuery {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub status: Option<String>,
}

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
struct BackendPaymentAdminApiResult<T: Serialize> {
    code: String,
    msg: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<T>,
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
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<BackendPaymentMethodView>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, method_key, display_name, provider_code,
                       status, sort_order, created_at, updated_at
                FROM commerce_payment_method
                WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT))
                  AND (? IS NULL OR LOWER(COALESCE(status, '')) = LOWER(CAST(? AS TEXT)))
                ORDER BY sort_order ASC, created_at ASC
                "#,
            )
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .bind(query.status.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment methods: {error}"))
            })?;

            Ok(rows.iter().map(map_method_row_sqlite).collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, account_no, provider_code, merchant_id, environment, country_code, settlement_currency, status FROM commerce_payment_provider_account WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list provider accounts: {error}")))?;
            Ok(rows
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
                .collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority FROM commerce_payment_channel WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY priority ASC, created_at ASC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list channels: {error}")))?;
            Ok(rows
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
                .collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at FROM commerce_payment_route_rule WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY priority ASC, created_at ASC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list route rules: {error}")))?;
            Ok(rows.iter().map(map_route_rule_sqlite).collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, payment_id, attempt_no, provider_code, channel_id, amount, currency, status, provider_transaction_id, created_at FROM commerce_payment_attempt WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list attempts: {error}")))?;
            Ok(rows.iter().map(map_attempt_sqlite).collect())
        })
    }

    fn list_webhook_events<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, event_id, provider_code, event_type, status, received_at, processed_at, retries FROM commerce_payment_webhook_event WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY received_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list webhook events: {error}")))?;
            Ok(rows.iter().map(map_webhook_event_sqlite).collect())
        })
    }

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let row = sqlx::query(
                "UPDATE commerce_payment_webhook_event SET status = 'queued', processed_at = NULL, retries = COALESCE(retries, 0) + 1, updated_at = ? WHERE event_id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) RETURNING id, event_id, provider_code, event_type, status, received_at, processed_at, retries",
            )
            .bind(&now)
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to replay webhook event: {error}")))?;
            Ok(map_webhook_event_sqlite(&row))
        })
    }

    fn list_reconciliation_runs<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, currency_code, created_at FROM commerce_payment_reconciliation_run WHERE tenant_id = CAST(? AS TEXT) AND (organization_id IS NULL AND ? IS NULL OR organization_id = CAST(? AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .bind(scope.organization_id.as_deref())
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list reconciliation runs: {error}")))?;
            Ok(rows.iter().map(map_reconciliation_run_sqlite).collect())
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
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<BackendPaymentMethodView>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
                SELECT id, tenant_id, organization_id, method_key, display_name, provider_code,
                       status, sort_order, created_at, updated_at
                FROM commerce_payment_method
                WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT))
                  AND ($3::text IS NULL OR LOWER(COALESCE(status, '')) = LOWER($3::text))
                ORDER BY sort_order ASC, created_at ASC
                "#,
            )
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.status.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| {
                CommerceServiceError::storage(format!("failed to list payment methods: {error}"))
            })?;

            Ok(rows.iter().map(map_method_row_pg).collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, account_no, provider_code, merchant_id, environment, country_code, settlement_currency, status FROM commerce_payment_provider_account WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list provider accounts: {error}")))?;
            Ok(rows
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
                .collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, channel_no, provider_account_id, method_id, scene_code, currency_code, country_code, status, priority FROM commerce_payment_channel WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY priority ASC, created_at ASC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list channels: {error}")))?;
            Ok(rows
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
                .collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, rule_no, priority, purchase_type, country_code, currency_code, client_platform, amount_min, amount_max, user_segment, risk_level, channel_id, status, starts_at, ends_at FROM commerce_payment_route_rule WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY priority ASC, created_at ASC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list route rules: {error}")))?;
            Ok(rows.iter().map(map_route_rule_pg).collect())
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
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, payment_id, attempt_no, provider_code, channel_id, amount, currency, status, provider_transaction_id, created_at FROM commerce_payment_attempt WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list attempts: {error}")))?;
            Ok(rows.iter().map(map_attempt_pg).collect())
        })
    }

    fn list_webhook_events<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, event_id, provider_code, event_type, status, received_at, processed_at, retries FROM commerce_payment_webhook_event WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY received_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list webhook events: {error}")))?;
            Ok(rows.iter().map(map_webhook_event_pg).collect())
        })
    }

    fn replay_webhook_event<'a>(
        &'a self,
        scope: BackendTenantScope,
        event_id: String,
    ) -> CommerceBackendPaymentAdminFuture<'a, serde_json::Value> {
        Box::pin(async move {
            let now = current_timestamp_string();
            let row = sqlx::query(
                "UPDATE commerce_payment_webhook_event SET status = 'queued', processed_at = NULL, retries = COALESCE(retries, 0) + 1, updated_at = $1 WHERE event_id = CAST($2 AS TEXT) AND tenant_id = CAST($3 AS TEXT) AND (organization_id IS NULL AND $4::text IS NULL OR organization_id = CAST($4 AS TEXT)) RETURNING id, event_id, provider_code, event_type, status, received_at, processed_at, retries",
            )
            .bind(&now)
            .bind(&event_id)
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_one(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to replay webhook event: {error}")))?;
            Ok(map_webhook_event_pg(&row))
        })
    }

    fn list_reconciliation_runs<'a>(
        &'a self,
        scope: BackendTenantScope,
    ) -> CommerceBackendPaymentAdminFuture<'a, Vec<serde_json::Value>> {
        Box::pin(async move {
            let rows = sqlx::query(
                "SELECT id, run_no, provider_code, provider_account_id, reconciliation_type, period_start, period_end, status, matched_count, mismatched_count, currency_code, created_at FROM commerce_payment_reconciliation_run WHERE tenant_id = CAST($1 AS TEXT) AND (organization_id IS NULL AND $2::text IS NULL OR organization_id = CAST($2 AS TEXT)) ORDER BY created_at DESC, id DESC",
            )
            .bind(&scope.tenant_id)
            .bind(scope.organization_id.as_deref())
            .fetch_all(&self.pool)
            .await
            .map_err(|error| CommerceServiceError::storage(format!("failed to list reconciliation runs: {error}")))?;
            Ok(rows.iter().map(map_reconciliation_run_pg).collect())
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

impl<T: Serialize> BackendPaymentAdminApiResult<T> {
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
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let query = BackendPaymentMethodListQuery {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        status: params.status,
    };

    match state.store.list_payment_methods(query).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(
            items.into_iter().map(map_method).collect::<Vec<_>>(),
        ))
        .into_response(),
        Err(error) => backend_payment_error_response("payment method list is unavailable", error),
    }
}

async fn create_method(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertPaymentMethodBody>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let write_headers =
        match validate_backend_write_payload(&headers, "payment-method-upsert", &body, "pm") {
            Ok(headers) => headers,
            Err(response) => return response,
        };
    let method_key = body.method_key.unwrap_or_default();
    let display_name = body.display_name.unwrap_or_else(|| method_key.clone());
    let provider_code = body.provider_code.unwrap_or_else(|| method_key.clone());
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
        Ok(view) => Json(BackendPaymentAdminApiResult::success(map_method(view))).into_response(),
        Err(error) => backend_payment_error_response("payment method upsert failed", error),
    }
}

async fn update_method(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Path(method_key): Path<String>,
    Json(body): Json<UpsertPaymentMethodBody>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let write_headers =
        match validate_backend_write_payload(&headers, "payment-method-upsert", &body, "pm") {
            Ok(headers) => headers,
            Err(response) => return response,
        };
    let command = UpsertBackendPaymentMethodCommand {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        method_key,
        display_name: body
            .display_name
            .unwrap_or_else(|| "payment method".to_owned()),
        provider_code: body
            .provider_code
            .unwrap_or_else(|| "wechat_pay".to_owned()),
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        sort_order: body.sort_order.unwrap_or(0),
        request_no: write_headers.request_no,
        idempotency_key: write_headers.idempotency_key,
    };

    match state.store.upsert_payment_method(command).await {
        Ok(view) => Json(BackendPaymentAdminApiResult::success(map_method(view))).into_response(),
        Err(error) => backend_payment_error_response("payment method upsert failed", error),
    }
}

async fn list_provider_accounts(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_provider_accounts(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment provider account list is unavailable", error)
        }
    }
}

async fn create_provider_account(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertProviderAccountBody>,
) -> Response {
    upsert_provider_account_inner(state, runtime_context, headers, None, body).await
}

async fn update_provider_account(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Path(provider_account_id): Path<String>,
    Json(body): Json<UpsertProviderAccountBody>,
) -> Response {
    upsert_provider_account_inner(
        state,
        runtime_context,
        headers,
        Some(provider_account_id),
        body,
    )
    .await
}

async fn upsert_provider_account_inner(
    state: BackendPaymentAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    provider_account_id: Option<String>,
    body: UpsertProviderAccountBody,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let _write_headers = match validate_backend_write_payload(
        &headers,
        "payment-provider-account-upsert",
        &body,
        "provider-account",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let account_no = body.account_no.unwrap_or_else(|| {
        provider_account_id
            .clone()
            .unwrap_or_else(|| "acct".to_owned())
    });
    let payload = BackendProviderAccountPayload {
        id: provider_account_id,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        account_no,
        provider_code: body
            .provider_code
            .unwrap_or_else(|| "wechat_pay".to_owned()),
        merchant_id: body.merchant_id.unwrap_or_else(|| "merchant".to_owned()),
        environment: body.environment.unwrap_or_else(|| "production".to_owned()),
        country_code: body.country_code.unwrap_or_else(|| "CN".to_owned()),
        settlement_currency: body.settlement_currency.unwrap_or_else(|| "CNY".to_owned()),
        secret_ref: body
            .secret_ref
            .unwrap_or_else(|| "vault://secret".to_owned()),
        webhook_secret_ref: body.webhook_secret_ref,
        certificate_ref: body.certificate_ref,
        status: body.status.unwrap_or_else(|| "active".to_owned()),
    };
    match state.store.upsert_provider_account(payload).await {
        Ok(item) => Json(BackendPaymentAdminApiResult::success(item)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment provider account upsert failed", error)
        }
    }
}

async fn list_channels(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_channels(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => backend_payment_error_response("payment channel list is unavailable", error),
    }
}

async fn create_channel(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertChannelBody>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let _write_headers = match validate_backend_write_payload(
        &headers,
        "payment-channel-upsert",
        &body,
        "payment-channel",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let channel_no = body.channel_no.unwrap_or_else(|| "channel".to_owned());
    let payload = BackendPaymentChannelPayload {
        id: None,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        channel_no: channel_no.clone(),
        provider_account_id: body
            .provider_account_id
            .unwrap_or_else(|| stable_storage_id(&["provider-account", &channel_no])),
        method_id: body
            .method_id
            .unwrap_or_else(|| stable_storage_id(&["payment-method", &channel_no])),
        scene_code: body.scene_code.unwrap_or_else(|| "app".to_owned()),
        currency_code: body.currency_code.unwrap_or_else(|| "CNY".to_owned()),
        country_code: body.country_code.unwrap_or_else(|| "CN".to_owned()),
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        priority: body.priority.unwrap_or(0),
    };
    match state.store.upsert_channel(payload).await {
        Ok(item) => Json(BackendPaymentAdminApiResult::success(item)).into_response(),
        Err(error) => backend_payment_error_response("payment channel upsert failed", error),
    }
}

async fn list_route_rules(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_route_rules(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment route rule list is unavailable", error)
        }
    }
}

async fn create_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<UpsertRouteRuleBody>,
) -> Response {
    upsert_route_rule_inner(state, runtime_context, headers, None, body).await
}

async fn update_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Path(route_rule_id): Path<String>,
    Json(body): Json<UpsertRouteRuleBody>,
) -> Response {
    upsert_route_rule_inner(state, runtime_context, headers, Some(route_rule_id), body).await
}

async fn upsert_route_rule_inner(
    state: BackendPaymentAdminState,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    route_rule_id: Option<String>,
    body: UpsertRouteRuleBody,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let _write_headers = match validate_backend_write_payload(
        &headers,
        "payment-route-rule-upsert",
        &body,
        "payment-route-rule",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let rule_no = body
        .rule_no
        .unwrap_or_else(|| route_rule_id.clone().unwrap_or_else(|| "rule".to_owned()));
    let payload = BackendRouteRulePayload {
        id: route_rule_id,
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        rule_no: rule_no.clone(),
        priority: body.priority.unwrap_or(0),
        purchase_type: body.purchase_type,
        country_code: body.country_code,
        currency_code: body.currency_code,
        client_platform: body.client_platform,
        amount_min: body.amount_min,
        amount_max: body.amount_max,
        user_segment: body.user_segment,
        risk_level: body.risk_level,
        channel_id: body
            .channel_id
            .unwrap_or_else(|| stable_storage_id(&["payment-channel", &rule_no])),
        status: body.status.unwrap_or_else(|| "active".to_owned()),
        starts_at: body.starts_at,
        ends_at: body.ends_at,
    };
    match state.store.upsert_route_rule(payload).await {
        Ok(item) => Json(BackendPaymentAdminApiResult::success(item)).into_response(),
        Err(error) => backend_payment_error_response("payment route rule upsert failed", error),
    }
}

async fn delete_route_rule(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(route_rule_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.delete_route_rule(scope, route_rule_id).await {
        Ok(()) => Json(BackendPaymentAdminApiResult::<()>::success(())).into_response(),
        Err(error) => backend_payment_error_response("payment route rule delete failed", error),
    }
}

async fn list_attempts(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_attempts(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => backend_payment_error_response("payment attempt list is unavailable", error),
    }
}

async fn list_webhook_events(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_webhook_events(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment webhook event list is unavailable", error)
        }
    }
}

async fn replay_webhook_event(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    Path(event_id): Path<String>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.replay_webhook_event(scope, event_id).await {
        Ok(item) => Json(BackendPaymentAdminApiResult::success(item)).into_response(),
        Err(error) => backend_payment_error_response("payment webhook replay failed", error),
    }
}

async fn list_reconciliation_runs(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let scope = BackendTenantScope {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
    };
    match state.store.list_reconciliation_runs(scope).await {
        Ok(items) => Json(BackendPaymentAdminApiResult::success(items)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment reconciliation run list is unavailable", error)
        }
    }
}

async fn create_reconciliation_run(
    State(state): State<BackendPaymentAdminState>,
    runtime_context: Option<Extension<IamAppContext>>,
    headers: HeaderMap,
    Json(body): Json<CreateReconciliationRunBody>,
) -> Response {
    let subject = match app_runtime_subject_from_extension(runtime_context) {
        Ok(subject) => subject,
        Err(message) => return unauthorized_response(message),
    };
    let write_headers = match validate_backend_write_payload(
        &headers,
        "payment-reconciliation-run-create",
        &body,
        "recon",
    ) {
        Ok(headers) => headers,
        Err(response) => return response,
    };
    let period_start = body
        .period_start
        .or(body.statement_date)
        .unwrap_or_else(current_timestamp_string);
    let period_end = body.period_end.unwrap_or_else(|| period_start.clone());
    let payload = BackendReconciliationRunPayload {
        tenant_id: subject.tenant_id,
        organization_id: subject.organization_id,
        provider_code: body
            .provider_code
            .unwrap_or_else(|| "wechat_pay".to_owned()),
        provider_account_id: body
            .provider_account_id
            .or(body.account_id)
            .unwrap_or_else(|| "account".to_owned()),
        reconciliation_type: body
            .reconciliation_type
            .unwrap_or_else(|| "daily".to_owned()),
        period_start,
        period_end,
        currency_code: body.currency_code.unwrap_or_else(|| "CNY".to_owned()),
        request_no: write_headers.request_no,
        idempotency_key: write_headers.idempotency_key,
    };
    match state.store.create_reconciliation_run(payload).await {
        Ok(item) => Json(BackendPaymentAdminApiResult::success(item)).into_response(),
        Err(error) => {
            backend_payment_error_response("payment reconciliation run create failed", error)
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

fn backend_payment_error_response(context: &str, error: CommerceServiceError) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(BackendPaymentAdminApiResult::<()>::error(
            "5000",
            format!("{context}: {}", error.message()),
        )),
    )
        .into_response()
}

fn unauthorized_response(message: impl Into<String>) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        Json(BackendPaymentAdminApiResult::<()>::error("4010", message)),
    )
        .into_response()
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
        "paymentId": sqlite_string(row, "payment_id"),
        "attemptNo": sqlite_string(row, "attempt_no"),
        "providerCode": sqlite_string(row, "provider_code"),
        "channelId": sqlite_string(row, "channel_id"),
        "amount": sqlite_string(row, "amount"),
        "currency": sqlite_string(row, "currency"),
        "status": sqlite_string(row, "status"),
        "providerTransactionId": sqlite_optional_string(row, "provider_transaction_id"),
        "createdAt": sqlite_string(row, "created_at"),
    })
}

fn map_attempt_pg(row: &PgRow) -> serde_json::Value {
    serde_json::json!({
        "id": pg_string(row, "id"),
        "paymentId": pg_string(row, "payment_id"),
        "attemptNo": pg_string(row, "attempt_no"),
        "providerCode": pg_string(row, "provider_code"),
        "channelId": pg_string(row, "channel_id"),
        "amount": pg_string(row, "amount"),
        "currency": pg_string(row, "currency"),
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
    headers: &HeaderMap,
    scope: &str,
    body: &impl Serialize,
    request_no_prefix: &str,
) -> Result<AppWriteCommandHeaders, Response> {
    validate_write_payload(headers, scope, body, |idempotency_key| {
        format!("{request_no_prefix}-{idempotency_key}")
    })
    .map_err(backend_write_header_error)
}

fn backend_write_header_error(error: WriteCommandHeaderError) -> Response {
    let message = match error {
        WriteCommandHeaderError::MissingHeader(name) => format!("{name} header is required"),
        WriteCommandHeaderError::InvalidHeader(message) => message.to_owned(),
    };
    (
        StatusCode::BAD_REQUEST,
        Json(BackendPaymentAdminApiResult::<()>::error("4001", message)),
    )
        .into_response()
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
