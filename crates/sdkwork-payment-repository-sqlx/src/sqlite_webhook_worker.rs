//! Processes queued `commerce_payment_webhook_event` rows (admin replay / background drain).

use sdkwork_contract_service::CommerceServiceError;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};

use crate::payment_attempt_context::load_payment_webhook_attempt_context_by_out_trade_no_sqlite;
use crate::shared::current_timestamp_string;
use crate::sqlite_webhook_ingestion::apply_webhook_payment_status_sqlite;

/// Maximum webhook events claimed per worker tick.
pub const WEBHOOK_BATCH_SIZE: i64 = 32;

/// Outcome of a single webhook processing pass.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebhookProcessSummary {
    pub claimed: usize,
    pub processed: usize,
    pub failed: usize,
    pub payment_attempt_contexts: Vec<crate::payment_attempt_context::PaymentWebhookAttemptContext>,
}

/// Claim queued webhook events and apply normalized payment status transitions.
pub async fn process_queued_webhook_events(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
) -> Result<WebhookProcessSummary, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT id, provider_code, payload
        FROM commerce_payment_webhook_event
        WHERE tenant_id = CAST(? AS TEXT)
          AND status = 'queued'
        ORDER BY received_at ASC, id ASC
        LIMIT ?
        "#,
    )
    .bind(tenant_id)
    .bind(WEBHOOK_BATCH_SIZE)
    .fetch_all(pool)
    .await
    .map_err(|error| {
        CommerceServiceError::storage(format!("failed to claim webhook events: {error}"))
    })?;

    let claimed = rows.len();
    let mut processed = 0usize;
    let mut failed = 0usize;
    let mut payment_attempt_contexts = Vec::new();

    for row in &rows {
        let event_id: String = row
            .try_get("id")
            .map_err(|error| CommerceServiceError::storage(format!("webhook event id: {error}")))?;
        let provider_code: String = row.try_get("provider_code").map_err(|error| {
            CommerceServiceError::storage(format!("webhook provider_code: {error}"))
        })?;
        let payload: String = row
            .try_get("payload")
            .map_err(|error| CommerceServiceError::storage(format!("webhook payload: {error}")))?;

        match process_queued_webhook_row(pool, tenant_id, &event_id, &provider_code, &payload).await
        {
            Ok(context) => {
                processed += 1;
                if let Some(context) = context {
                    payment_attempt_contexts.push(context);
                }
            }
            Err(_) => failed += 1,
        }
    }

    Ok(WebhookProcessSummary {
        claimed,
        processed,
        failed,
        payment_attempt_contexts,
    })
}

async fn process_queued_webhook_row(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    event_id: &str,
    provider_code: &str,
    payload: &str,
) -> Result<Option<crate::payment_attempt_context::PaymentWebhookAttemptContext>, CommerceServiceError> {
    let parsed: Value = serde_json::from_str(payload).map_err(|error| {
        CommerceServiceError::storage(format!("webhook payload json invalid: {error}"))
    })?;
    let normalized = parsed.get("normalized");
    let out_trade_no = normalized
        .and_then(|value| value.get("outTradeNo"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            parsed
                .get("out_trade_no")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let payment_status = normalized
        .and_then(|value| value.get("paymentStatus"))
        .and_then(Value::as_str)
        .map(str::to_owned)
        .or_else(|| {
            parsed
                .get("payment_status")
                .or_else(|| parsed.get("trade_status"))
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    if let (Some(out_trade_no), Some(payment_status)) =
        (out_trade_no.as_deref(), payment_status.as_deref())
    {
        let now = current_timestamp_string();
        let mut tx = pool.begin().await.map_err(|error| {
            CommerceServiceError::storage(format!("failed to begin queued webhook transaction: {error}"))
        })?;
        let (_, applied_status) = apply_webhook_payment_status_sqlite(
            &mut tx,
            provider_code,
            Some(out_trade_no),
            Some(payment_status),
            &now,
        )
        .await?;
        let payment_attempt_context = if applied_status.as_deref() == Some("succeeded") {
            load_payment_webhook_attempt_context_by_out_trade_no_sqlite(&mut tx, out_trade_no).await?
        } else {
            None
        };
        sqlx::query(
            r#"
            UPDATE commerce_payment_webhook_event
            SET status = 'processed', processed_at = ?, updated_at = ?
            WHERE id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT)
            "#,
        )
        .bind(&now)
        .bind(&now)
        .bind(event_id)
        .bind(tenant_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            CommerceServiceError::storage(format!("failed to mark queued webhook processed: {error}"))
        })?;
        tx.commit().await.map_err(|error| {
            CommerceServiceError::storage(format!("failed to commit queued webhook transaction: {error}"))
        })?;
        return Ok(payment_attempt_context);
    }

    let now = current_timestamp_string();
    let mut tx = pool.begin().await.map_err(|error| {
        CommerceServiceError::storage(format!("failed to begin queued webhook transaction: {error}"))
    })?;
    apply_webhook_payment_status_sqlite(
        &mut tx,
        provider_code,
        out_trade_no.as_deref(),
        payment_status.as_deref(),
        &now,
    )
    .await?;
    sqlx::query(
        r#"
        UPDATE commerce_payment_webhook_event
        SET status = 'processed', processed_at = ?, updated_at = ?
        WHERE id = CAST(? AS TEXT) AND tenant_id = CAST(? AS TEXT)
        "#,
    )
    .bind(&now)
    .bind(&now)
    .bind(event_id)
    .bind(tenant_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| {
        CommerceServiceError::storage(format!("failed to mark queued webhook processed: {error}"))
    })?;
    tx.commit().await.map_err(|error| {
        CommerceServiceError::storage(format!("failed to commit queued webhook transaction: {error}"))
    })?;
    Ok(None)
}
