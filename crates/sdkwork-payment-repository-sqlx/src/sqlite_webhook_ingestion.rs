use sdkwork_contract_service::CommerceServiceError;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};

use crate::payment_attempt_context::{
    load_attempt_by_out_trade_no_sqlite, load_owner_order_settlement_scope_by_out_trade_no_sqlite,
    OwnerOrderSettlementScope,
};
use crate::shared::{current_timestamp_string, stable_storage_id, store_error};
use crate::webhook_status::map_provider_payment_status;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestProviderWebhookCommand {
    pub provider_code: String,
    pub provider_event_id: String,
    pub event_type: Option<String>,
    pub out_trade_no: Option<String>,
    pub payment_status: Option<String>,
    pub payload: Value,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestProviderWebhookOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub payment_attempt_id: Option<String>,
    pub applied_status: Option<String>,
    pub settlement_scope: Option<OwnerOrderSettlementScope>,
}

pub fn empty_ingest_outcome(
    webhook_event_id: String,
    replayed: bool,
) -> IngestProviderWebhookOutcome {
    IngestProviderWebhookOutcome {
        webhook_event_id,
        replayed,
        payment_attempt_id: None,
        applied_status: None,
        settlement_scope: None,
    }
}

pub async fn ingest_provider_webhook_sqlite(
    pool: &Pool<Sqlite>,
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

    let mut tx = pool.begin().await.map_err(|error| {
        store_error("failed to begin webhook ingestion transaction", error)
    })?;

    let out_trade_no = command.out_trade_no.as_deref().unwrap_or("").trim();
    if out_trade_no.is_empty() {
        return Ok(empty_ingest_outcome(event_id, false));
    }

    let (tenant_id, _) = match load_attempt_by_out_trade_no_sqlite(&mut tx, out_trade_no).await? {
        Some(context) => context,
        None => {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit webhook without matching attempt", error)
            })?;
            return Ok(empty_ingest_outcome(event_id, false));
        }
    };

    let insert = sqlx::query(
        r#"
        INSERT INTO commerce_payment_webhook_event
            (id, tenant_id, event_id, event_type, provider_code, payload, status, received_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, 'queued', ?, ?, ?)
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
    .bind(&now)
    .bind(&now)
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to insert webhook event", error))?;

    if insert.rows_affected() == 0 {
        tx.commit().await.map_err(|error| {
            store_error("failed to commit webhook idempotency replay", error)
        })?;
        return Ok(empty_ingest_outcome(event_id, true));
    }

    let (payment_attempt_id, applied_status) = apply_webhook_payment_status_sqlite(
        &mut tx,
        &command.provider_code,
        command.out_trade_no.as_deref(),
        command.payment_status.as_deref(),
        &now,
    )
    .await?;

    let settlement_scope = if applied_status.as_deref() == Some("succeeded") {
        load_owner_order_settlement_scope_by_out_trade_no_sqlite(&mut tx, out_trade_no).await?
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE commerce_payment_webhook_event
        SET status = 'processed', processed_at = ?, updated_at = ?
        WHERE id = CAST(? AS TEXT)
        "#,
    )
    .bind(&now)
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
        settlement_scope,
    })
}

pub(crate) async fn apply_webhook_payment_status_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
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
        WHERE out_trade_no = CAST(? AS TEXT)
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
        SET status = ?, updated_at = ?, paid_at = CASE WHEN ? = 'succeeded' THEN ? ELSE paid_at END
        WHERE id = CAST(? AS TEXT)
        "#,
    )
    .bind(target_status)
    .bind(now)
    .bind(target_status)
    .bind(now)
    .bind(&attempt_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to update payment attempt from webhook", error))?;

    sqlx::query(
        r#"
        UPDATE commerce_payment_intent
        SET status = ?, updated_at = ?
        WHERE id = (
            SELECT payment_intent_id FROM commerce_payment_attempt WHERE id = CAST(? AS TEXT)
        )
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

#[cfg(test)]
mod sqlite_webhook_ingestion_tests {
    use sdkwork_order_service::PayOwnerOrderCommand;
    use serde_json::json;

    use crate::{
        sqlite_owner_order_payment::SqliteCommerceOwnerOrderPaymentStore,
        test_sqlite_pool::payment_store_e2e_sqlite_memory_pool,
    };

    async fn seed_pending_recharge_order(pool: &sqlx::SqlitePool) -> String {
        let now = "2026-07-05T12:00:00Z";
        sqlx::query(
            r#"
            INSERT INTO commerce_order
                (id, tenant_id, owner_user_id, order_no, status, subject, currency_code,
                 payment_status, created_at, updated_at)
            VALUES ('order-rch-1', '100001', 'user-1', 'RCH-1', 'pending_payment', 'points_recharge', 'CNY',
                    'pending', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed recharge order");

        sqlx::query(
            r#"
            INSERT INTO commerce_order_amount_breakdown
                (id, tenant_id, order_id, allocation_type, payable_amount, discount_amount, created_at)
            VALUES ('breakdown-rch-1', '100001', 'order-rch-1', 'order_total', '9.90', '0.00', ?)
            "#,
        )
        .bind(now)
        .execute(pool)
        .await
        .expect("seed breakdown");

        sqlx::query(
            r#"
            INSERT INTO commerce_payment_method
                (id, tenant_id, method_key, display_name, provider_code, status, sort_order,
                 idempotency_key, created_at, updated_at)
            VALUES ('pm-wechat', '100001', 'wechat_pay', 'WeChat Pay', 'wechat_pay', 'active', 0,
                    'pm-wechat-seed', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed payment method");

        let payments = SqliteCommerceOwnerOrderPaymentStore::new(pool.clone());
        let pay = PayOwnerOrderCommand::new(
            "100001",
            None,
            "user-1",
            "order-rch-1",
            "wechat_pay",
            "req-pay-1",
            "idem-pay-1",
        )
        .expect("pay command");
        let outcome = payments.pay_owner_order(pay).await.expect("pay owner order");
        outcome.out_trade_no
    }

    #[tokio::test]
    async fn webhook_ingest_marks_attempt_succeeded_and_returns_settlement_scope() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        let out_trade_no = seed_pending_recharge_order(&pool).await;

        let outcome = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "wechat_pay".to_owned(),
                provider_event_id: "evt-1".to_owned(),
                event_type: Some("payment.success".to_owned()),
                out_trade_no: Some(out_trade_no),
                payment_status: Some("SUCCESS".to_owned()),
                payload: json!({ "result_code": "SUCCESS" }),
            },
        )
        .await
        .expect("ingest webhook");

        assert!(!outcome.replayed);
        assert_eq!(outcome.applied_status.as_deref(), Some("succeeded"));
        let scope = outcome
            .settlement_scope
            .expect("settlement scope for succeeded recharge");
        assert_eq!(scope.order_id, "order-rch-1");
        assert_eq!(scope.order_subject.as_deref(), Some("points_recharge"));

        let status: String = sqlx::query_scalar(
            "SELECT status FROM commerce_payment_attempt WHERE order_id = 'order-rch-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("attempt status");
        assert_eq!(status, "succeeded");
    }
}
