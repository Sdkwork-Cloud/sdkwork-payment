use std::collections::BTreeMap;

use sdkwork_contract_service::CommerceServiceError;
use serde_json::json;
use sqlx::{Pool, Postgres, Row, Sqlite};

use crate::shared::{current_timestamp_string, store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentAttemptProviderContext {
    pub attempt_id: String,
    pub provider_code: String,
    pub out_trade_no: String,
    pub amount: String,
    pub tenant_id: String,
    pub idempotency_key: String,
    pub provider_transaction_id: Option<String>,
}

fn provider_transaction_id_from_callback_payload(payload: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(payload).ok()?;
    let raw = value
        .get("providerTransactionId")
        .or_else(|| value.get("provider_transaction_id"))?;
    let text = raw.as_str()?.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_owned())
    }
}

fn merge_callback_payload_patch(
    existing: &str,
    payment_params: &BTreeMap<String, String>,
) -> String {
    let mut value = serde_json::from_str(existing).unwrap_or_else(|_| json!({}));
    let Some(obj) = value.as_object_mut() else {
        return existing.to_owned();
    };
    if let Some(native_id) = payment_params.get("providerTransactionId") {
        obj.insert("providerTransactionId".to_owned(), json!(native_id));
    }
    if let Some(status) = payment_params.get("providerStatus") {
        obj.insert("providerStatus".to_owned(), json!(status));
    }
    value.to_string()
}

pub async fn persist_attempt_enrichment_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    attempt_id: &str,
    payment_params: &BTreeMap<String, String>,
) -> Result<(), CommerceServiceError> {
    if payment_params.get("providerTransactionId").is_none()
        && payment_params.get("providerStatus").is_none()
    {
        return Ok(());
    }
    let row = sqlx::query(
        r#"
        SELECT callback_payload
        FROM commerce_payment_attempt
        WHERE id = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
        "#,
    )
    .bind(attempt_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt callback payload", error))?;
    let Some(row) = row else {
        return Ok(());
    };
    let existing = string_cell(&row, "callback_payload");
    let merged = merge_callback_payload_patch(&existing, payment_params);
    let now = current_timestamp_string();
    sqlx::query(
        r#"
        UPDATE commerce_payment_attempt
        SET callback_payload = ?, updated_at = ?
        WHERE id = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
        "#,
    )
    .bind(&merged)
    .bind(&now)
    .bind(attempt_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .map_err(|error| store_error("failed to persist payment attempt enrichment", error))?;
    Ok(())
}

pub async fn persist_attempt_enrichment_postgres(
    pool: &Pool<Postgres>,
    tenant_id: &str,
    attempt_id: &str,
    payment_params: &BTreeMap<String, String>,
) -> Result<(), CommerceServiceError> {
    if payment_params.get("providerTransactionId").is_none()
        && payment_params.get("providerStatus").is_none()
    {
        return Ok(());
    }
    let row = sqlx::query(
        r#"
        SELECT callback_payload
        FROM commerce_payment_attempt
        WHERE id = CAST($1 AS TEXT)
          AND tenant_id = CAST($2 AS TEXT)
        "#,
    )
    .bind(attempt_id)
    .bind(tenant_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt callback payload", error))?;
    let Some(row) = row else {
        return Ok(());
    };
    let existing = string_cell(&row, "callback_payload");
    let merged = merge_callback_payload_patch(&existing, payment_params);
    let now = current_timestamp_string();
    sqlx::query(
        r#"
        UPDATE commerce_payment_attempt
        SET callback_payload = $1, updated_at = $2
        WHERE id = CAST($3 AS TEXT)
          AND tenant_id = CAST($4 AS TEXT)
        "#,
    )
    .bind(&merged)
    .bind(&now)
    .bind(attempt_id)
    .bind(tenant_id)
    .execute(pool)
    .await
    .map_err(|error| store_error("failed to persist payment attempt enrichment", error))?;
    Ok(())
}

pub async fn load_payment_attempt_provider_context_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    owner_user_id: &str,
    payment_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id, callback_payload, idempotency_key
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST(? AS TEXT)
          AND owner_user_id = CAST(? AS TEXT)
          AND id = CAST(? AS TEXT)
        "#,
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(payment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt provider context", error))?;

    Ok(row.map(|row| {
        let callback_payload = string_cell(&row, "callback_payload");
        PaymentAttemptProviderContext {
            attempt_id: string_cell(&row, "id"),
            provider_code: string_cell(&row, "provider_code"),
            out_trade_no: string_cell(&row, "out_trade_no"),
            amount: string_cell(&row, "amount"),
            tenant_id: string_cell(&row, "tenant_id"),
            idempotency_key: string_cell(&row, "idempotency_key"),
            provider_transaction_id: provider_transaction_id_from_callback_payload(
                &callback_payload,
            ),
        }
    }))
}

pub async fn load_payment_attempt_provider_context_by_id_sqlite(
    pool: &Pool<Sqlite>,
    payment_attempt_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id, callback_payload, idempotency_key
        FROM commerce_payment_attempt
        WHERE id = CAST(? AS TEXT)
        "#,
    )
    .bind(payment_attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        store_error(
            "failed to load payment attempt provider context by id",
            error,
        )
    })?;

    Ok(row.map(|row| {
        let callback_payload = string_cell(&row, "callback_payload");
        PaymentAttemptProviderContext {
            attempt_id: string_cell(&row, "id"),
            provider_code: string_cell(&row, "provider_code"),
            out_trade_no: string_cell(&row, "out_trade_no"),
            amount: string_cell(&row, "amount"),
            tenant_id: string_cell(&row, "tenant_id"),
            idempotency_key: string_cell(&row, "idempotency_key"),
            provider_transaction_id: provider_transaction_id_from_callback_payload(
                &callback_payload,
            ),
        }
    }))
}

pub async fn load_payment_attempt_provider_context_postgres(
    pool: &Pool<Postgres>,
    tenant_id: &str,
    owner_user_id: &str,
    payment_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id, callback_payload, idempotency_key
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST($1 AS TEXT)
          AND owner_user_id = CAST($2 AS TEXT)
          AND id = CAST($3 AS TEXT)
        "#,
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(payment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt provider context", error))?;

    Ok(row.map(|row| {
        let callback_payload = string_cell(&row, "callback_payload");
        PaymentAttemptProviderContext {
            attempt_id: string_cell(&row, "id"),
            provider_code: string_cell(&row, "provider_code"),
            out_trade_no: string_cell(&row, "out_trade_no"),
            amount: string_cell(&row, "amount"),
            tenant_id: string_cell(&row, "tenant_id"),
            idempotency_key: string_cell(&row, "idempotency_key"),
            provider_transaction_id: provider_transaction_id_from_callback_payload(
                &callback_payload,
            ),
        }
    }))
}

pub async fn load_payment_attempt_provider_context_by_id_postgres(
    pool: &Pool<Postgres>,
    payment_attempt_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id, callback_payload, idempotency_key
        FROM commerce_payment_attempt
        WHERE id = CAST($1 AS TEXT)
        "#,
    )
    .bind(payment_attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        store_error(
            "failed to load payment attempt provider context by id",
            error,
        )
    })?;

    Ok(row.map(|row| {
        let callback_payload = string_cell(&row, "callback_payload");
        PaymentAttemptProviderContext {
            attempt_id: string_cell(&row, "id"),
            provider_code: string_cell(&row, "provider_code"),
            out_trade_no: string_cell(&row, "out_trade_no"),
            amount: string_cell(&row, "amount"),
            tenant_id: string_cell(&row, "tenant_id"),
            idempotency_key: string_cell(&row, "idempotency_key"),
            provider_transaction_id: provider_transaction_id_from_callback_payload(
                &callback_payload,
            ),
        }
    }))
}

/// Payment-attempt context returned after webhook ingest (no order-table join).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentWebhookAttemptContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

pub(crate) async fn load_payment_webhook_attempt_context_by_out_trade_no_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    out_trade_no: &str,
) -> Result<Option<PaymentWebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, owner_user_id, order_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve payment webhook attempt context", error))?;

    Ok(row.map(|row| PaymentWebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        owner_user_id: string_cell(&row, "owner_user_id"),
        order_id: string_cell(&row, "order_id"),
    }))
}

pub(crate) async fn load_payment_webhook_attempt_context_by_out_trade_no_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    out_trade_no: &str,
) -> Result<Option<PaymentWebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, owner_user_id, order_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve payment webhook attempt context", error))?;

    Ok(row.map(|row| PaymentWebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        owner_user_id: string_cell(&row, "owner_user_id"),
        order_id: string_cell(&row, "order_id"),
    }))
}

pub(crate) async fn load_attempt_by_out_trade_no_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    out_trade_no: &str,
) -> Result<Option<(String, String)>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve webhook tenant", error))?;

    Ok(row.map(|row| (string_cell(&row, "tenant_id"), string_cell(&row, "id"))))
}

pub(crate) async fn load_attempt_by_out_trade_no_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    out_trade_no: &str,
) -> Result<Option<(String, String)>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve webhook tenant", error))?;

    Ok(row.map(|row| (string_cell(&row, "tenant_id"), string_cell(&row, "id"))))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebhookAttemptContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub provider_code: String,
}

pub async fn load_webhook_attempt_context_by_out_trade_no_sqlite(
    pool: &Pool<Sqlite>,
    out_trade_no: &str,
) -> Result<Option<WebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load webhook attempt context", error))?;

    Ok(row.map(|row| WebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    }))
}

pub async fn load_webhook_attempt_context_by_out_trade_no_postgres(
    pool: &Pool<Postgres>,
    out_trade_no: &str,
) -> Result<Option<WebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load webhook attempt context", error))?;

    Ok(row.map(|row| WebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    }))
}
