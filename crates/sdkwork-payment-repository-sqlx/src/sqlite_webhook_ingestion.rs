use sdkwork_contract_service::CommerceServiceError;
use serde_json::Value;
use sqlx::{Pool, Row, Sqlite};

use crate::payment_attempt_context::{
    load_attempt_by_out_trade_no_sqlite, load_payment_webhook_attempt_context_by_id_sqlite,
    PaymentWebhookAttemptContext, PaymentWebhookAttemptIdentity,
};
use crate::shared::{current_timestamp_string, store_error, string_cell};
use crate::webhook_event_payload::{
    build_stored_webhook_payload, parse_stored_webhook_payload, provider_scoped_webhook_event_id,
    validate_stored_webhook_scope, webhook_event_storage_id, StoredWebhookPayload,
    WebhookEventInsert, WebhookEventPayloadInput, WEBHOOK_EVENT_STATUS_FAILED,
    WEBHOOK_EVENT_STATUS_PROCESSED, WEBHOOK_EVENT_STATUS_QUEUED,
};
use crate::webhook_status::map_provider_payment_status;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestProviderWebhookCommand {
    pub provider_code: String,
    pub provider_event_id: String,
    pub event_type: Option<String>,
    pub out_trade_no: Option<String>,
    pub payment_status: Option<String>,
    pub payload: Value,
    /// Scope for persisting unmatched events when `out_trade_no` cannot be resolved.
    pub tenant_id: Option<String>,
    pub organization_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IngestProviderWebhookOutcome {
    pub webhook_event_id: String,
    pub replayed: bool,
    pub payment_attempt_id: Option<String>,
    pub applied_status: Option<String>,
    pub payment_attempt_context: Option<PaymentWebhookAttemptContext>,
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
        payment_attempt_context: None,
    }
}

async fn persist_webhook_event_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    insert: &WebhookEventInsert<'_>,
) -> Result<bool, CommerceServiceError> {
    let result = sqlx::query(
        r#"
        INSERT INTO commerce_payment_webhook_event
            (id, tenant_id, organization_id, event_id, event_type, provider_code, payload, status,
             last_error, received_at, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT (tenant_id, event_id) DO NOTHING
        "#,
    )
    .bind(insert.internal_id)
    .bind(insert.tenant_id)
    .bind(insert.organization_id)
    .bind(insert.provider_scoped_event_id)
    .bind(insert.event_type)
    .bind(insert.provider_code)
    .bind(insert.payload_json)
    .bind(insert.status)
    .bind(insert.last_error)
    .bind(insert.now)
    .bind(insert.now)
    .bind(insert.now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert webhook event", error))?;
    Ok(result.rows_affected() == 1)
}

pub async fn ingest_provider_webhook_sqlite(
    pool: &Pool<Sqlite>,
    command: IngestProviderWebhookCommand,
) -> Result<IngestProviderWebhookOutcome, CommerceServiceError> {
    let provider_code = command.provider_code.trim().to_ascii_lowercase();
    if provider_code.is_empty() {
        return Err(CommerceServiceError::validation(
            "payment webhook provider code is required",
        ));
    }
    let provider_event_id = command.provider_event_id.trim();
    if provider_event_id.is_empty() {
        return Err(CommerceServiceError::validation(
            "payment webhook provider event id is required",
        ));
    }
    let now = current_timestamp_string();
    let mut tx = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|error| store_error("failed to begin webhook ingestion transaction", error))?;
    let out_trade_no = command
        .out_trade_no
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let command_tenant_id = command
        .tenant_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let command_organization_id = command
        .organization_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if command_tenant_id.is_none() && command_organization_id.is_some() {
        return Err(CommerceServiceError::validation(
            "payment webhook organization scope requires tenant scope",
        ));
    }
    let attempt_identity = match out_trade_no {
        Some(out_trade_no) => {
            load_attempt_by_out_trade_no_sqlite(
                &mut tx,
                &provider_code,
                out_trade_no,
                command_tenant_id,
                command_organization_id,
            )
            .await?
        }
        None => None,
    };
    let tenant_id = attempt_identity
        .as_ref()
        .map(|identity| identity.tenant_id.clone())
        .or_else(|| command_tenant_id.map(str::to_owned))
        .ok_or_else(|| {
            CommerceServiceError::conflict(
                "payment webhook tenant scope could not be resolved safely",
            )
        })?;
    let organization_id = attempt_identity
        .as_ref()
        .and_then(|identity| identity.organization_id.clone())
        .or_else(|| command_organization_id.map(str::to_owned));
    let provider_scoped_event_id =
        provider_scoped_webhook_event_id(&provider_code, provider_event_id);
    let internal_id = webhook_event_storage_id(&tenant_id, &provider_scoped_event_id);
    let unmatched_reason = if attempt_identity.is_none() {
        Some(if out_trade_no.is_some() {
            "payment_attempt_not_found"
        } else {
            "out_trade_no_missing"
        })
    } else {
        None
    };
    let payload_json = build_stored_webhook_payload(WebhookEventPayloadInput {
        provider_code: &provider_code,
        provider_event_id,
        provider_scoped_event_id: &provider_scoped_event_id,
        event_type: command.event_type.as_deref(),
        out_trade_no,
        payment_status: command.payment_status.as_deref(),
        provider_payload: &command.payload,
        attempt_identity: attempt_identity.as_ref(),
        unmatched_reason,
    })?;

    let proposed_event_status = if attempt_identity.is_some() {
        WEBHOOK_EVENT_STATUS_QUEUED
    } else {
        WEBHOOK_EVENT_STATUS_FAILED
    };
    let insert = WebhookEventInsert {
        internal_id: &internal_id,
        tenant_id: &tenant_id,
        organization_id: organization_id.as_deref(),
        provider_scoped_event_id: &provider_scoped_event_id,
        event_type: command.event_type.as_deref().unwrap_or("payment"),
        provider_code: &provider_code,
        payload_json: &payload_json,
        status: proposed_event_status,
        last_error: unmatched_reason,
        now: &now,
    };
    let inserted = persist_webhook_event_sqlite(&mut tx, &insert).await?;
    let (attempt_identity, payment_status) = if inserted {
        (attempt_identity, command.payment_status.clone())
    } else {
        let stored = load_existing_webhook_event_sqlite(
            &mut tx,
            &tenant_id,
            organization_id.as_deref(),
            &provider_scoped_event_id,
        )
        .await?;
        (stored.attempt_identity, stored.payment_status)
    };
    let Some(attempt_identity) = attempt_identity else {
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit unmatched webhook event", error))?;
        return Ok(empty_ingest_outcome(internal_id, !inserted));
    };
    let applied_status = apply_webhook_payment_status_sqlite(
        &mut tx,
        &attempt_identity,
        payment_status.as_deref(),
        &now,
    )
    .await?;
    let payment_attempt_id = Some(attempt_identity.payment_attempt_id.clone());

    let payment_attempt_context = if applied_status.as_deref() == Some("succeeded") {
        load_payment_webhook_attempt_context_by_id_sqlite(
            &mut tx,
            &attempt_identity.payment_attempt_id,
            &attempt_identity.provider_code,
            Some(&attempt_identity.tenant_id),
            attempt_identity.organization_id.as_deref(),
        )
        .await?
    } else {
        None
    };

    sqlx::query(
        r#"
        UPDATE commerce_payment_webhook_event
        SET status = ?, processed_at = ?, updated_at = ?, last_error = NULL
        WHERE id = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
        "#,
    )
    .bind(WEBHOOK_EVENT_STATUS_PROCESSED)
    .bind(&now)
    .bind(&now)
    .bind(&internal_id)
    .bind(&tenant_id)
    .bind(organization_id.as_deref())
    .bind(organization_id.as_deref())
    .execute(&mut *tx)
    .await
    .map_err(|error| store_error("failed to mark webhook event processed", error))?;

    tx.commit()
        .await
        .map_err(|error| store_error("failed to commit webhook ingestion transaction", error))?;

    Ok(IngestProviderWebhookOutcome {
        webhook_event_id: internal_id,
        replayed: !inserted,
        payment_attempt_id,
        applied_status,
        payment_attempt_context,
    })
}

async fn load_existing_webhook_event_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    provider_scoped_event_id: &str,
) -> Result<StoredWebhookPayload, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT provider_code, payload
        FROM commerce_payment_webhook_event
        WHERE tenant_id = CAST(? AS TEXT)
          AND event_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
        "#,
    )
    .bind(tenant_id)
    .bind(provider_scoped_event_id)
    .bind(organization_id)
    .bind(organization_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load existing webhook event", error))?
    .ok_or_else(|| {
        CommerceServiceError::conflict(
            "webhook idempotency identity conflicts with another organization scope",
        )
    })?;
    let provider_code = string_cell(&row, "provider_code");
    let payload = string_cell(&row, "payload");
    let stored = parse_stored_webhook_payload(&payload)?;
    validate_stored_webhook_scope(
        &stored,
        provider_scoped_event_id,
        &provider_code,
        tenant_id,
        organization_id,
    )?;
    Ok(stored)
}

pub(crate) async fn apply_webhook_payment_status_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    identity: &PaymentWebhookAttemptIdentity,
    payment_status: Option<&str>,
    now: &str,
) -> Result<Option<String>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, payment_intent_id, tenant_id, organization_id, owner_user_id, order_id, status
        FROM commerce_payment_attempt
        WHERE id = CAST(? AS TEXT)
          AND payment_intent_id = CAST(? AS TEXT)
          AND provider_code = CAST(? AS TEXT)
          AND out_trade_no = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND owner_user_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND deleted_at IS NULL
        "#,
    )
    .bind(&identity.payment_attempt_id)
    .bind(&identity.payment_intent_id)
    .bind(&identity.provider_code)
    .bind(&identity.out_trade_no)
    .bind(&identity.tenant_id)
    .bind(identity.organization_id.as_deref())
    .bind(identity.organization_id.as_deref())
    .bind(&identity.owner_user_id)
    .bind(&identity.order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load exact payment attempt for webhook", error))?
    .ok_or_else(|| {
        CommerceServiceError::conflict("stored webhook payment attempt identity no longer exists")
    })?;

    let attempt_id = string_cell(&row, "id");
    let payment_intent_id = string_cell(&row, "payment_intent_id");
    let resolved_tenant_id = string_cell(&row, "tenant_id");
    let resolved_organization_id: Option<String> = row.try_get("organization_id").ok().flatten();
    let owner_user_id = string_cell(&row, "owner_user_id");
    let order_id = string_cell(&row, "order_id");
    let Some(raw_status) = payment_status.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let Some(target_status) = map_provider_payment_status(&identity.provider_code, raw_status)
    else {
        return Ok(None);
    };

    let order_row = sqlx::query(
        r#"
        SELECT id
        FROM commerce_order
        WHERE tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND owner_user_id = CAST(? AS TEXT)
          AND id = CAST(? AS TEXT)
        "#,
    )
    .bind(&resolved_tenant_id)
    .bind(resolved_organization_id.as_deref())
    .bind(resolved_organization_id.as_deref())
    .bind(&owner_user_id)
    .bind(&order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to lock webhook owner order", error))?;
    if order_row.is_none() {
        return Err(CommerceServiceError::storage(
            "payment webhook attempt references a missing or deleted order",
        ));
    }

    let identity_rows = sqlx::query(
        r#"
        SELECT id, payment_intent_id
        FROM commerce_payment_attempt
        WHERE provider_code = CAST(? AS TEXT)
          AND out_trade_no = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND deleted_at IS NULL
        ORDER BY id
        LIMIT 2
        "#,
    )
    .bind(&identity.provider_code)
    .bind(&identity.out_trade_no)
    .bind(&resolved_tenant_id)
    .bind(resolved_organization_id.as_deref())
    .bind(resolved_organization_id.as_deref())
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| store_error("failed to revalidate webhook identity", error))?;
    match identity_rows.as_slice() {
        [identity]
            if string_cell(identity, "id") == attempt_id
                && string_cell(identity, "payment_intent_id") == payment_intent_id => {}
        [] => {
            return Err(CommerceServiceError::storage(
                "payment webhook attempt disappeared during settlement",
            ));
        }
        _ => {
            return Err(CommerceServiceError::conflict(
                "multiple payment attempts match webhook identity",
            ));
        }
    }

    let intent_row = sqlx::query(
        r#"
        SELECT status
        FROM commerce_payment_intent
        WHERE id = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND owner_user_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND deleted_at IS NULL
        "#,
    )
    .bind(&payment_intent_id)
    .bind(&resolved_tenant_id)
    .bind(resolved_organization_id.as_deref())
    .bind(resolved_organization_id.as_deref())
    .bind(&owner_user_id)
    .bind(&order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to lock webhook payment intent", error))?
    .ok_or_else(|| {
        CommerceServiceError::storage(
            "payment webhook attempt references a missing or deleted payment intent",
        )
    })?;
    let intent_status = string_cell(&intent_row, "status");

    let attempt_row = sqlx::query(
        r#"
        SELECT status
        FROM commerce_payment_attempt
        WHERE id = CAST(? AS TEXT)
          AND payment_intent_id = CAST(? AS TEXT)
          AND provider_code = CAST(? AS TEXT)
          AND out_trade_no = CAST(? AS TEXT)
          AND tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND owner_user_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND deleted_at IS NULL
        "#,
    )
    .bind(&attempt_id)
    .bind(&payment_intent_id)
    .bind(&identity.provider_code)
    .bind(&identity.out_trade_no)
    .bind(&resolved_tenant_id)
    .bind(resolved_organization_id.as_deref())
    .bind(resolved_organization_id.as_deref())
    .bind(&owner_user_id)
    .bind(&order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to lock webhook payment attempt", error))?
    .ok_or_else(|| {
        CommerceServiceError::storage("payment webhook attempt changed identity during settlement")
    })?;
    let attempt_status = string_cell(&attempt_row, "status");

    if !intent_status.eq_ignore_ascii_case(target_status) {
        crate::shared::ensure_payment_status_transition(&intent_status, target_status)?;
        let updated = sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = ?, updated_at = ?
            WHERE id = CAST(? AS TEXT)
              AND tenant_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            "#,
        )
        .bind(target_status)
        .bind(now)
        .bind(&payment_intent_id)
        .bind(&resolved_tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| store_error("failed to update payment intent from webhook", error))?;
        if updated.rows_affected() != 1 {
            return Err(CommerceServiceError::storage(
                "webhook payment intent update did not affect exactly one row",
            ));
        }
    }

    if !attempt_status.eq_ignore_ascii_case(target_status) {
        crate::shared::ensure_payment_status_transition(&attempt_status, target_status)?;
        let updated = sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = ?, updated_at = ?, paid_at = CASE
                WHEN ? = 'succeeded' THEN COALESCE(NULLIF(paid_at, ''), ?)
                ELSE paid_at
            END
            WHERE id = CAST(? AS TEXT)
              AND payment_intent_id = CAST(? AS TEXT)
              AND tenant_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            "#,
        )
        .bind(target_status)
        .bind(now)
        .bind(target_status)
        .bind(now)
        .bind(&attempt_id)
        .bind(&payment_intent_id)
        .bind(&resolved_tenant_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| store_error("failed to update payment attempt from webhook", error))?;
        if updated.rows_affected() != 1 {
            return Err(CommerceServiceError::storage(
                "webhook payment attempt update did not affect exactly one row",
            ));
        }
    }

    Ok(Some(target_status.to_owned()))
}

#[cfg(test)]
mod sqlite_webhook_ingestion_tests {
    use sdkwork_payment_service::{PayOwnerOrderCommand, PayOwnerOrderCommandInput};
    use serde_json::json;
    use sqlx::Row;

    use crate::{
        replay_stored_webhook_event_sqlite,
        sqlite_owner_order_payment::SqliteCommerceOwnerOrderPaymentStore,
        test_sqlite_pool::payment_store_e2e_sqlite_memory_pool, WebhookStoredReplayScope,
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
            VALUES ('breakdown-rch-1', '100001', 'order-rch-1', 'order_total', '990', '0', ?)
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
        let pay = PayOwnerOrderCommand::new(PayOwnerOrderCommandInput {
            tenant_id: "100001".to_owned(),
            organization_id: None,
            owner_user_id: "user-1".to_owned(),
            order_id: "order-rch-1".to_owned(),
            payment_method: "wechat_pay".to_owned(),
            payment_scene: None,
            payment_attempt_callback_payload: None,
            request_no: "req-pay-1".to_owned(),
            idempotency_key: "idem-pay-1".to_owned(),
        })
        .expect("pay command");
        let outcome = payments
            .pay_owner_order(pay)
            .await
            .expect("pay owner order");
        outcome.out_trade_no
    }

    async fn seed_scoped_attempt(pool: &sqlx::SqlitePool) {
        let now = "2026-07-12T03:00:00Z";
        sqlx::query(
            r#"
            INSERT INTO commerce_order
                (id, tenant_id, organization_id, owner_user_id, order_no, status, subject,
                 currency_code, payment_status, created_at, updated_at)
            VALUES ('order-scoped-1', 'tenant-scoped', 'org-scoped', 'user-scoped', 'ORD-SCOPED-1',
                    'pending_payment', 'test', 'CNY', 'pending', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed scoped order");
        sqlx::query(
            r#"
            INSERT INTO commerce_payment_intent
                (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                 provider_code, amount, status, idempotency_key, created_at, updated_at)
            VALUES ('intent-scoped-1', 'tenant-scoped', 'org-scoped', 'user-scoped',
                    'order-scoped-1', 'PI-SCOPED-1', 'stripe', '100', 'pending',
                    'intent-scoped-idem-1', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed scoped intent");
        sqlx::query(
            r#"
            INSERT INTO commerce_payment_attempt
                (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
                 payment_method, provider_code, out_trade_no, amount, status, idempotency_key,
                 created_at, updated_at)
            VALUES ('attempt-scoped-1', 'tenant-scoped', 'org-scoped', 'user-scoped',
                    'intent-scoped-1', 'order-scoped-1', 'stripe', 'stripe', 'trade-scoped-1',
                    '100', 'pending', 'attempt-scoped-idem-1', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(pool)
        .await
        .expect("seed scoped attempt");
    }

    #[tokio::test]
    async fn webhook_ingest_marks_attempt_succeeded_and_returns_payment_attempt_context() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        let out_trade_no = seed_pending_recharge_order(&pool).await;

        let outcome = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "wechat_pay".to_owned(),
                provider_event_id: "evt-1".to_owned(),
                event_type: Some("payment.success".to_owned()),
                out_trade_no: Some(out_trade_no.clone()),
                payment_status: Some("SUCCESS".to_owned()),
                payload: json!({ "result_code": "SUCCESS" }),
                tenant_id: None,
                organization_id: None,
            },
        )
        .await
        .expect("ingest webhook");

        assert!(!outcome.replayed);
        assert_eq!(outcome.applied_status.as_deref(), Some("succeeded"));
        let context = outcome
            .payment_attempt_context
            .expect("payment attempt context for succeeded recharge");
        assert!(!context.payment_attempt_id.is_empty());
        assert_eq!(context.out_trade_no, out_trade_no);
        assert_eq!(context.order_id, "order-rch-1");
        assert_eq!(context.owner_user_id, "user-1");

        let status: String = sqlx::query_scalar(
            "SELECT status FROM commerce_payment_attempt WHERE order_id = 'order-rch-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("attempt status");
        assert_eq!(status, "succeeded");
    }

    #[tokio::test]
    async fn webhook_ingest_rejects_cross_tenant_duplicate_out_trade_number() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        let now = "2026-07-12T02:00:00Z";
        for (suffix, tenant, user, order) in [
            ("a", "tenant-a", "user-a", "order-webhook-a"),
            ("b", "tenant-b", "user-b", "order-webhook-b"),
        ] {
            sqlx::query(
                r#"
                INSERT INTO commerce_order
                    (id, tenant_id, owner_user_id, order_no, status, subject, currency_code,
                     payment_status, created_at, updated_at)
                VALUES (?, ?, ?, ?, 'pending_payment', 'test', 'CNY', 'pending', ?, ?)
                "#,
            )
            .bind(order)
            .bind(tenant)
            .bind(user)
            .bind(format!("ORD-WEBHOOK-{suffix}"))
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .expect("seed duplicate webhook order");

            sqlx::query(
                r#"
                INSERT INTO commerce_payment_intent
                    (id, tenant_id, owner_user_id, order_id, payment_intent_no, amount, status,
                     idempotency_key, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, '100', 'pending', ?, ?, ?)
                "#,
            )
            .bind(format!("pi-webhook-{suffix}"))
            .bind(tenant)
            .bind(user)
            .bind(order)
            .bind(format!("PI-WEBHOOK-{suffix}"))
            .bind(format!("pi-webhook-idem-{suffix}"))
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .expect("seed duplicate webhook intent");

            sqlx::query(
                r#"
                INSERT INTO commerce_payment_attempt
                    (id, tenant_id, owner_user_id, payment_intent_id, order_id, payment_method,
                     provider_code, out_trade_no, amount, status, idempotency_key, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, 'wechat_pay', 'wechat_pay', 'duplicate-out-trade', '100',
                        'pending', ?, ?, ?)
                "#,
            )
            .bind(format!("pa-webhook-{suffix}"))
            .bind(tenant)
            .bind(user)
            .bind(format!("pi-webhook-{suffix}"))
            .bind(order)
            .bind(format!("pa-webhook-idem-{suffix}"))
            .bind(now)
            .bind(now)
            .execute(&pool)
            .await
            .expect("seed duplicate webhook attempt");
        }

        let error = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "wechat_pay".to_owned(),
                provider_event_id: "evt-duplicate-out-trade".to_owned(),
                event_type: Some("payment.success".to_owned()),
                out_trade_no: Some("duplicate-out-trade".to_owned()),
                payment_status: Some("SUCCESS".to_owned()),
                payload: json!({ "result_code": "SUCCESS" }),
                tenant_id: None,
                organization_id: None,
            },
        )
        .await
        .expect_err("ambiguous cross-tenant webhook must fail closed");

        assert_eq!(error.code(), "conflict");
    }

    #[tokio::test]
    async fn unmatched_webhooks_are_provider_tenant_and_organization_scoped() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        let mut outcomes = Vec::new();
        for (tenant_id, organization_id, provider_code) in [
            ("tenant-a", "org-a", "stripe"),
            ("tenant-a", "org-a", "wechat_pay"),
            ("tenant-b", "org-b", "stripe"),
        ] {
            outcomes.push(
                super::ingest_provider_webhook_sqlite(
                    &pool,
                    super::IngestProviderWebhookCommand {
                        provider_code: provider_code.to_owned(),
                        provider_event_id: "same-provider-event".to_owned(),
                        event_type: Some("payment.unknown".to_owned()),
                        out_trade_no: Some("missing-trade".to_owned()),
                        payment_status: Some("pending".to_owned()),
                        payload: json!({"provider": provider_code}),
                        tenant_id: Some(tenant_id.to_owned()),
                        organization_id: Some(organization_id.to_owned()),
                    },
                )
                .await
                .expect("persist unmatched webhook"),
            );
        }

        assert_ne!(outcomes[0].webhook_event_id, outcomes[1].webhook_event_id);
        assert_ne!(outcomes[0].webhook_event_id, outcomes[2].webhook_event_id);
        let rows = sqlx::query(
            r#"
            SELECT tenant_id, organization_id, event_id, provider_code, status, last_error, payload
            FROM commerce_payment_webhook_event
            ORDER BY tenant_id, provider_code
            "#,
        )
        .fetch_all(&pool)
        .await
        .expect("list unmatched webhook events");
        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert_eq!(row.get::<String, _>("status"), "failed");
            assert_eq!(
                row.get::<String, _>("last_error"),
                "payment_attempt_not_found"
            );
            assert_ne!(row.get::<String, _>("event_id"), "same-provider-event");
            let payload: serde_json::Value = serde_json::from_str(&row.get::<String, _>("payload"))
                .expect("stored unmatched payload");
            assert_eq!(payload["normalized"]["matchState"], "unmatched");
        }

        let tenant_a_stripe = rows
            .iter()
            .find(|row| {
                row.get::<String, _>("tenant_id") == "tenant-a"
                    && row.get::<String, _>("provider_code") == "stripe"
            })
            .expect("tenant-a stripe webhook");
        let error = replay_stored_webhook_event_sqlite(
            &pool,
            WebhookStoredReplayScope {
                tenant_id: "tenant-a".to_owned(),
                organization_id: Some("org-a".to_owned()),
            },
            tenant_a_stripe.get("event_id"),
        )
        .await
        .expect_err("unmatched webhook replay must fail closed");
        assert_eq!(error.code(), "conflict");
    }

    #[tokio::test]
    async fn unmatched_webhook_without_tenant_scope_fails_without_persistence() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        let error = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "stripe".to_owned(),
                provider_event_id: "evt-no-scope".to_owned(),
                event_type: Some("payment.unknown".to_owned()),
                out_trade_no: Some("missing-trade".to_owned()),
                payment_status: Some("pending".to_owned()),
                payload: json!({"id": "evt-no-scope"}),
                tenant_id: None,
                organization_id: None,
            },
        )
        .await
        .expect_err("unmatched webhook without trusted tenant scope must fail closed");
        assert_eq!(error.code(), "conflict");

        let count =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM commerce_payment_webhook_event")
                .fetch_one(&pool)
                .await
                .expect("count webhook events");
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn duplicate_webhook_replays_stored_attempt_and_status() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        seed_scoped_attempt(&pool).await;
        let now = "2026-07-12T03:05:00Z";
        sqlx::query(
            r#"
            INSERT INTO commerce_payment_attempt
                (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
                 payment_method, provider_code, out_trade_no, amount, status, idempotency_key,
                 created_at, updated_at)
            VALUES ('attempt-scoped-other', 'tenant-scoped', 'org-scoped', 'user-scoped',
                    'intent-scoped-1', 'order-scoped-1', 'stripe', 'stripe', 'trade-scoped-other',
                    '100', 'pending', 'attempt-scoped-other-idem', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed second scoped attempt");

        let first = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "stripe".to_owned(),
                provider_event_id: "evt-stored-fact".to_owned(),
                event_type: Some("payment.succeeded".to_owned()),
                out_trade_no: Some("trade-scoped-1".to_owned()),
                payment_status: Some("succeeded".to_owned()),
                payload: json!({"id": "evt-stored-fact", "status": "succeeded"}),
                tenant_id: Some("tenant-scoped".to_owned()),
                organization_id: Some("org-scoped".to_owned()),
            },
        )
        .await
        .expect("ingest original webhook");
        assert!(!first.replayed);

        let replay = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "stripe".to_owned(),
                provider_event_id: "evt-stored-fact".to_owned(),
                event_type: Some("payment.canceled".to_owned()),
                out_trade_no: Some("trade-scoped-other".to_owned()),
                payment_status: Some("canceled".to_owned()),
                payload: json!({"id": "evt-stored-fact", "status": "canceled"}),
                tenant_id: Some("tenant-scoped".to_owned()),
                organization_id: Some("org-scoped".to_owned()),
            },
        )
        .await
        .expect("duplicate webhook must replay the persisted normalized fact");

        assert!(replay.replayed);
        assert_eq!(
            replay.payment_attempt_id.as_deref(),
            Some("attempt-scoped-1")
        );
        assert_eq!(replay.applied_status.as_deref(), Some("succeeded"));
        let original_status: String = sqlx::query_scalar(
            "SELECT status FROM commerce_payment_attempt WHERE id = 'attempt-scoped-1'",
        )
        .fetch_one(&pool)
        .await
        .expect("load original attempt status");
        let other_status: String = sqlx::query_scalar(
            "SELECT status FROM commerce_payment_attempt WHERE id = 'attempt-scoped-other'",
        )
        .fetch_one(&pool)
        .await
        .expect("load other attempt status");
        assert_eq!(original_status, "succeeded");
        assert_eq!(other_status, "pending");

        let payload: String = sqlx::query_scalar(
            "SELECT payload FROM commerce_payment_webhook_event WHERE id = CAST(? AS TEXT)",
        )
        .bind(&first.webhook_event_id)
        .fetch_one(&pool)
        .await
        .expect("load stored webhook payload");
        let payload: serde_json::Value =
            serde_json::from_str(&payload).expect("parse stored webhook payload");
        assert_eq!(payload["normalized"]["outTradeNo"], "trade-scoped-1");
        assert_eq!(payload["normalized"]["paymentStatus"], "succeeded");
    }

    #[tokio::test]
    async fn matched_webhook_persists_exact_identity_and_replay_rejects_identity_drift() {
        let pool = payment_store_e2e_sqlite_memory_pool().await;
        seed_scoped_attempt(&pool).await;
        let outcome = super::ingest_provider_webhook_sqlite(
            &pool,
            super::IngestProviderWebhookCommand {
                provider_code: "stripe".to_owned(),
                provider_event_id: "evt-scoped-1".to_owned(),
                event_type: Some("payment.succeeded".to_owned()),
                out_trade_no: Some("trade-scoped-1".to_owned()),
                payment_status: Some("succeeded".to_owned()),
                payload: json!({"id": "evt-scoped-1"}),
                tenant_id: None,
                organization_id: None,
            },
        )
        .await
        .expect("ingest scoped webhook");
        assert_eq!(
            outcome.payment_attempt_id.as_deref(),
            Some("attempt-scoped-1")
        );

        let row = sqlx::query(
            r#"
            SELECT event_id, organization_id, status, payload
            FROM commerce_payment_webhook_event
            WHERE id = CAST(? AS TEXT)
            "#,
        )
        .bind(&outcome.webhook_event_id)
        .fetch_one(&pool)
        .await
        .expect("load scoped webhook event");
        assert_eq!(row.get::<String, _>("organization_id"), "org-scoped");
        assert_eq!(row.get::<String, _>("status"), "processed");
        let event_id: String = row.get("event_id");
        let payload: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("payload")).expect("stored matched payload");
        assert_eq!(
            payload["normalized"]["attempt"]["paymentAttemptId"],
            "attempt-scoped-1"
        );
        assert_eq!(
            payload["normalized"]["attempt"]["paymentIntentId"],
            "intent-scoped-1"
        );

        let now = "2026-07-12T03:10:00Z";
        sqlx::query(
            r#"
            INSERT INTO commerce_payment_attempt
                (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
                 payment_method, provider_code, out_trade_no, amount, status, idempotency_key,
                 created_at, updated_at)
            VALUES ('attempt-scoped-2', 'tenant-scoped', 'org-scoped', 'user-scoped',
                    'intent-scoped-1', 'order-scoped-1', 'stripe', 'stripe', 'trade-scoped-1',
                    '100', 'pending', 'attempt-scoped-idem-2', ?, ?)
            "#,
        )
        .bind(now)
        .bind(now)
        .execute(&pool)
        .await
        .expect("seed competing attempt");
        let scope = WebhookStoredReplayScope {
            tenant_id: "tenant-scoped".to_owned(),
            organization_id: Some("org-scoped".to_owned()),
        };
        let error = replay_stored_webhook_event_sqlite(&pool, scope.clone(), event_id.clone())
            .await
            .expect_err("multiple active attempts must fail replay");
        assert_eq!(error.code(), "conflict");

        sqlx::query(
            "UPDATE commerce_payment_attempt SET deleted_at = ? WHERE id = 'attempt-scoped-2'",
        )
        .bind(now)
        .execute(&pool)
        .await
        .expect("soft delete competing attempt");
        replay_stored_webhook_event_sqlite(&pool, scope.clone(), event_id.clone())
            .await
            .expect("exact attempt replay after competing attempt removal");

        sqlx::query(
            "UPDATE commerce_payment_attempt SET deleted_at = ? WHERE id = 'attempt-scoped-1'",
        )
        .bind(now)
        .execute(&pool)
        .await
        .expect("soft delete exact attempt");
        let error = replay_stored_webhook_event_sqlite(&pool, scope, event_id)
            .await
            .expect_err("soft-deleted exact attempt must fail replay");
        assert_eq!(error.code(), "conflict");
    }
}
