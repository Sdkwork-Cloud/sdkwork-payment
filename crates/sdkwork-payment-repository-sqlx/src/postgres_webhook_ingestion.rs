use sdkwork_contract_service::CommerceServiceError;
use sqlx::{Pool, Postgres, Row};

use crate::payment_attempt_context::{
    load_attempt_by_out_trade_no_postgres, load_payment_webhook_attempt_context_by_out_trade_no_postgres,
};
use crate::shared::{current_timestamp_string, stable_storage_id, store_error};
use crate::sqlite_webhook_ingestion::{
    empty_ingest_outcome, IngestProviderWebhookCommand, IngestProviderWebhookOutcome,
};
use crate::webhook_status::map_provider_payment_status;

pub async fn ingest_provider_webhook_postgres(
    pool: &Pool<Postgres>,
    command: IngestProviderWebhookCommand,
) -> Result<IngestProviderWebhookOutcome, CommerceServiceError> {
    let now = current_timestamp_string();
    let event_id = stable_storage_id(&[
        "webhook",
        &command.provider_code,
        &command.provider_event_id,
    ]);
    let payload_json = serde_json::to_string(&serde_json::json!({
        "normalized": {
            "outTradeNo": command.out_trade_no,
            "paymentStatus": command.payment_status,
            "eventType": command.event_type,
            "providerEventId": command.provider_event_id,
        },
        "providerPayload": command.payload,
    }))
        .map_err(|error| CommerceServiceError::storage(format!("webhook payload json: {error}")))?;

    let out_trade_no = command.out_trade_no.as_deref().unwrap_or("").trim();
    if out_trade_no.is_empty() {
        if let Some(tenant_id) = command.tenant_id.as_deref().filter(|value| !value.is_empty()) {
            let mut tx = pool.begin().await.map_err(|error| {
                store_error("failed to begin unmatched webhook ingestion transaction", error)
            })?;
            sqlx::query(
                r#"
                INSERT INTO commerce_payment_webhook_event
                    (id, tenant_id, event_id, event_type, provider_code, payload, status, received_at, created_at, updated_at)
                VALUES ($1, $2, $3, $4, $5, $6, 'unmatched', $7, $8, $9)
                ON CONFLICT (tenant_id, event_id) DO NOTHING
                "#,
            )
            .bind(&event_id)
            .bind(tenant_id)
            .bind(&command.provider_event_id)
            .bind(command.event_type.as_deref().unwrap_or("payment"))
            .bind(&command.provider_code)
            .bind(&payload_json)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .execute(&mut *tx)
            .await
            .map_err(|error| store_error("failed to insert unmatched webhook event", error))?;
            tx.commit().await.map_err(|error| {
                store_error("failed to commit unmatched webhook ingestion transaction", error)
            })?;
            return Ok(empty_ingest_outcome(event_id, false));
        }
        return Ok(empty_ingest_outcome(event_id, false));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|error| store_error("failed to begin webhook ingestion transaction", error))?;

    let (tenant_id, _) = match load_attempt_by_out_trade_no_postgres(&mut tx, out_trade_no).await? {
        Some(context) => context,
        None => {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit webhook without matching attempt", error))?;
            return Ok(empty_ingest_outcome(event_id, false));
        }
    };

    let insert = sqlx::query(
        r#"
        INSERT INTO commerce_payment_webhook_event
            (id, tenant_id, event_id, event_type, provider_code, payload, status, received_at, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, 'queued', $7, $7, $7)
        ON CONFLICT (tenant_id, event_id) DO NOTHING
        "#,
    )
    .bind(&event_id)
    .bind(&tenant_id)
    .bind(&command.provider_event_id)
    .bind(command.event_type.as_deref().unwrap_or("payment"))
    .bind(&command.provider_code)
    .bind(&payload_json)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to insert webhook event", error))?;

    if insert.rows_affected() == 0 {
        let (payment_attempt_id, applied_status) = apply_webhook_payment_status_postgres(
            &mut tx,
            &command.provider_code,
            command.out_trade_no.as_deref(),
            command.payment_status.as_deref(),
            &now,
        )
        .await?;

        let payment_attempt_context = if applied_status.as_deref() == Some("succeeded") {
            load_payment_webhook_attempt_context_by_out_trade_no_postgres(&mut tx, out_trade_no)
                .await?
        } else {
            None
        };

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit webhook idempotency replay", error))?;
        return Ok(IngestProviderWebhookOutcome {
            webhook_event_id: event_id,
            replayed: true,
            payment_attempt_id,
            applied_status,
            payment_attempt_context,
        });
    }

    let (payment_attempt_id, applied_status) = apply_webhook_payment_status_postgres(
        &mut tx,
        &command.provider_code,
        command.out_trade_no.as_deref(),
        command.payment_status.as_deref(),
        &now,
    )
    .await?;

    let payment_attempt_context = if applied_status.as_deref() == Some("succeeded") {
        load_payment_webhook_attempt_context_by_out_trade_no_postgres(&mut tx, out_trade_no).await?
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE commerce_payment_webhook_event
        SET status = 'processed', processed_at = $1, updated_at = $1
        WHERE id = CAST($2 AS TEXT)
        "#,
    )
    .bind(&now)
    .bind(&event_id)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to mark webhook event processed", error))?;

    tx.commit()
        .await
        .map_err(|error| store_error("failed to commit webhook ingestion transaction", error))?;

    Ok(IngestProviderWebhookOutcome {
        webhook_event_id: event_id,
        replayed: false,
        payment_attempt_id,
        applied_status,
        payment_attempt_context,
    })
}

pub(crate) async fn apply_webhook_payment_status_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    provider_code: &str,
    out_trade_no: Option<&str>,
    payment_status: Option<&str>,
    now: &str,
) -> Result<(Option<String>, Option<String>), CommerceServiceError> {
    let Some(out_trade_no) = out_trade_no.filter(|value| !value.trim().is_empty()) else {
        return Ok((None, None));
    };
    let Some(raw_status) = payment_status.filter(|value| !value.trim().is_empty()) else {
        return Ok((None, None));
    };
    let Some(target_status) = map_provider_payment_status(provider_code, raw_status) else {
        return Ok((None, None));
    };

    let row = sqlx::query(
        r#"
        SELECT id, status
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load payment attempt for webhook", error))?;

    let Some(row) = row else {
        return Ok((None, None));
    };

    let attempt_id: String = row
        .try_get("id")
        .map_err(|error| CommerceServiceError::storage(format!("attempt id: {error}")))?;
    let current_status: String = row
        .try_get("status")
        .map_err(|error| CommerceServiceError::storage(format!("attempt status: {error}")))?;

    if current_status.eq_ignore_ascii_case(target_status) {
        return Ok((Some(attempt_id), Some(target_status.to_owned())));
    }

    crate::shared::ensure_payment_status_transition(&current_status, target_status)?;

    sqlx::query(
        r#"
        UPDATE commerce_payment_attempt
        SET status = $1, updated_at = $2, paid_at = CASE WHEN $1 = 'succeeded' THEN $2 ELSE paid_at END
        WHERE id = CAST($3 AS TEXT)
        "#,
    )
    .bind(target_status)
    .bind(now)
    .bind(&attempt_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update payment attempt from webhook", error))?;

    sqlx::query(
        r#"
        UPDATE commerce_payment_intent pi
        SET status = $1, updated_at = $2
        WHERE pi.id = (
            SELECT payment_intent_id FROM commerce_payment_attempt WHERE id = CAST($3 AS TEXT)
        )
          AND LOWER(COALESCE(pi.status, '')) IN ('created', 'pending', 'processing', 'refunding')
        "#,
    )
    .bind(target_status)
    .bind(now)
    .bind(&attempt_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update payment intent from webhook", error))?;

    Ok((Some(attempt_id), Some(target_status.to_owned())))
}
