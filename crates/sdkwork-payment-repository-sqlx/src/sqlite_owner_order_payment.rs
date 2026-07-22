use sdkwork_contract_service::{CommerceMoney, CommercePaymentStatus, CommerceServiceError};
use sdkwork_payment_service::{
    CancelOrderPaymentsCommand, ConfirmOwnerOrderPaymentOutcome, OrderPaymentSettlementAttempt,
    PayOwnerOrderCommand, PayOwnerOrderOutcome,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::order_reference::order_status_is_payable;
use crate::owner_payment_params::owner_order_payment_params;
use crate::payment_channel::select_payment_channel_sqlite;
use crate::shared::{
    current_timestamp_string, ensure_confirmation_intent_update,
    ensure_owner_payment_idempotency_replay_matches, payment_attempt_is_terminal_success,
    required_persisted_paid_at, resolve_confirmation_attempt_replayed, stable_storage_id,
};

#[derive(Debug, Clone)]
pub struct SqliteCommerceOwnerOrderPaymentStore {
    pool: SqlitePool,
}

impl SqliteCommerceOwnerOrderPaymentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn cancel_order_payments(
        &self,
        command: CancelOrderPaymentsCommand,
    ) -> Result<(), CommerceServiceError> {
        let now = current_timestamp_string();
        // Lock order, intents, and attempts in a stable order before canceling.
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| {
                store_error(
                    "failed to begin cancel owner order payment transaction",
                    error,
                )
            })?;

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
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            store_error("failed to lock owner order for payment cancellation", error)
        })?;
        if order_row.is_none() {
            return Err(CommerceServiceError::not_found(
                "owner order was not found for payment cancellation",
            ));
        }

        sqlx::query(
            r#"
            SELECT id
            FROM commerce_payment_intent
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            ORDER BY id
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner payment intents", error))?;

        sqlx::query(
            r#"
            SELECT id
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            ORDER BY id
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner payment attempts", error))?;

        let affected_intent = sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close order payment intents", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close order payment attempts", error))?;

        tx.commit().await.map_err(|error| {
            store_error(
                "failed to commit cancel owner order payment transaction",
                error,
            )
        })?;

        // Report a terminal payment instead of silently treating cancellation as successful.
        if affected_intent.rows_affected() == 0 {
            let existing = sqlx::query(
                r#"
                SELECT 1
                FROM commerce_payment_intent
                WHERE tenant_id = CAST(? AS TEXT)
                  AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
                  AND owner_user_id = CAST(? AS TEXT)
                  AND order_id = CAST(? AS TEXT)
                  AND deleted_at IS NULL
                  AND LOWER(COALESCE(status, '')) NOT IN ('created', 'pending', 'processing', 'canceled')
                LIMIT 1
                "#,
            )
            .bind(&command.tenant_id)
            .bind(&command.organization_id)
            .bind(&command.organization_id)
            .bind(&command.owner_user_id)
            .bind(&command.order_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| store_error("failed to verify cancel owner order payment state", error))?;

            if existing.is_some() {
                return Err(CommerceServiceError::conflict(
                    "order payment is in a terminal state and cannot be canceled",
                ));
            }
        }

        Ok(())
    }

    pub async fn pay_owner_order(
        &self,
        command: PayOwnerOrderCommand,
    ) -> Result<PayOwnerOrderOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| {
                store_error("failed to begin owner order payment transaction", error)
            })?;

        let order_row = sqlx::query(
            r#"
            SELECT
                o.id AS order_id,
                o.order_no AS order_sn,
                o.subject AS order_subject,
                o.status,
                COALESCE(
                    (
                        SELECT b.payable_amount
                        FROM commerce_order_amount_breakdown b
                        WHERE b.tenant_id = o.tenant_id
                          AND b.order_id = o.id
                          AND b.allocation_type = 'order_total'
                        LIMIT 1
                    ),
                    '0'
                ) AS total_amount
            FROM commerce_order o
            WHERE o.id = CAST(? AS TEXT)
              AND o.tenant_id = CAST(? AS TEXT)
              AND ((o.organization_id = CAST(? AS TEXT)) OR (o.organization_id IS NULL AND ? IS NULL))
              AND o.owner_user_id = CAST(? AS TEXT)
            "#,
        )
        .bind(&command.order_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner order for payment", error))?;

        let Some(order_row) = order_row else {
            return Err(CommerceServiceError::not_found("order was not found"));
        };
        let order_status = string_cell(&order_row, "status");
        let order_sn = string_cell(&order_row, "order_sn");
        let order_subject = optional_string_cell(&order_row, "order_subject");
        let total_amount = CommerceMoney::new(&string_cell(&order_row, "total_amount"))
            .map_err(CommerceServiceError::storage)?;
        if !order_status_is_payable(&order_status) {
            return Err(CommerceServiceError::conflict(
                "order is not pending payment",
            ));
        }

        if let Some(existing) = load_owner_payment_outcome_by_idempotency_in_tx(
            &mut tx,
            &command,
            &order_sn,
            order_subject.as_deref(),
        )
        .await?
        {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit idempotent owner payment replay", error)
            })?;
            return Ok(existing);
        }

        if let Some(existing) = load_reusable_owner_payment_in_tx(
            &mut tx,
            &command,
            &order_sn,
            order_subject.as_deref(),
        )
        .await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit reusable owner payment", error))?;
            return Ok(existing);
        }

        let channel = select_payment_channel_sqlite(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.payment_method,
            "CNY",
            total_amount.as_str(),
            command.payment_scene.as_deref(),
        )
        .await?;
        let now = current_timestamp_string();
        let payment_intent_id = stable_storage_id(&[
            "pi",
            &command.tenant_id,
            &command.order_id,
            &command.idempotency_key,
        ]);
        let payment_attempt_id = stable_storage_id(&[
            "pa",
            &command.tenant_id,
            &command.order_id,
            &command.idempotency_key,
        ]);
        let out_trade_no = format!(
            "OT-{}-{}",
            order_sn,
            &command.idempotency_key[..command.idempotency_key.len().min(24)]
        );

        sqlx::query(
            r#"
            INSERT INTO commerce_payment_intent
                (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                 payment_method, provider_code, amount, currency_code, status, request_no,
                 idempotency_key, created_at, updated_at)
            VALUES
                (?, CAST(? AS TEXT), CAST(? AS TEXT), CAST(? AS TEXT), ?, ?, ?, ?, ?, 'CNY', ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&payment_intent_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .bind(format!("PAY-{}", order_sn))
        .bind(&command.payment_method)
        .bind(&channel.provider_code)
        .bind(total_amount.as_str())
        .bind(CommercePaymentStatus::Pending.as_str())
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert owner order payment intent", error))?;

        if let Some(existing) = load_owner_payment_outcome_by_idempotency_in_tx(
            &mut tx,
            &command,
            &order_sn,
            order_subject.as_deref(),
        )
        .await?
        {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit idempotent owner payment replay", error)
            })?;
            return Ok(existing);
        }

        let callback_payload = command
            .payment_attempt_callback_payload
            .as_deref()
            .unwrap_or("{}");

        sqlx::query(
            r#"
            INSERT INTO commerce_payment_attempt
                (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
                 payment_method, provider_code, channel_id, out_trade_no, amount, currency_code, status,
                 callback_payload, request_no, idempotency_key, created_at, paid_at, updated_at)
            VALUES
                (?, CAST(? AS TEXT), CAST(? AS TEXT), CAST(? AS TEXT), ?, ?, ?, ?, ?, ?, ?, 'CNY', ?, ?, ?, ?, ?, NULL, ?)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&payment_attempt_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&payment_intent_id)
        .bind(&command.order_id)
        .bind(&command.payment_method)
        .bind(&channel.provider_code)
        .bind(channel.channel_id.as_deref())
        .bind(&out_trade_no)
        .bind(total_amount.as_str())
        .bind(CommercePaymentStatus::Pending.as_str())
        .bind(callback_payload)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert owner order payment attempt", error))?;

        let outcome = load_owner_payment_outcome_by_idempotency_in_tx(
            &mut tx,
            &command,
            &order_sn,
            order_subject.as_deref(),
        )
        .await?
        .ok_or_else(|| {
            CommerceServiceError::storage(
                "owner order payment attempt was not persisted after insert",
            )
        })?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit owner order payment transaction", error)
        })?;

        Ok(outcome)
    }

    pub async fn confirm_owner_order_payment(
        &self,
        settlement: &OrderPaymentSettlementAttempt,
    ) -> Result<ConfirmOwnerOrderPaymentOutcome, CommerceServiceError> {
        let confirmation_paid_at = current_timestamp_string();
        let tenant_id = settlement.tenant_id.as_str();
        let organization_id = settlement.organization_id.as_deref();
        let owner_user_id = settlement.owner_user_id.as_str();
        let order_id = settlement.order_id.as_str();
        let payment_attempt_id = settlement
            .payment_attempt_id
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let out_trade_no = settlement
            .out_trade_no
            .as_deref()
            .filter(|value| !value.trim().is_empty());
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| {
                store_error(
                    "failed to begin owner order payment confirmation transaction",
                    error,
                )
            })?;

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
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            store_error("failed to lock owner order for payment confirmation", error)
        })?;
        if order_row.is_none() {
            return Err(CommerceServiceError::not_found(
                "owner order was not found for payment confirmation",
            ));
        }

        let candidate_rows = sqlx::query(
            r#"
            SELECT id, payment_intent_id, status, paid_at, out_trade_no
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND (? IS NULL OR id = CAST(? AS TEXT))
              AND (? IS NULL OR out_trade_no = CAST(? AS TEXT))
              AND deleted_at IS NULL
            ORDER BY id
            LIMIT 2
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .bind(payment_attempt_id)
        .bind(payment_attempt_id)
        .bind(out_trade_no)
        .bind(out_trade_no)
        .fetch_all(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load owner order payment attempt", error))?;

        let candidate_row = match candidate_rows.as_slice() {
            [] => {
                return Err(CommerceServiceError::not_found(
                    "owner order payment attempt was not found",
                ));
            }
            [_first, _second, ..] => {
                return Err(CommerceServiceError::conflict(
                    "multiple payment attempts match manual confirmation",
                ));
            }
            [row] => row,
        };

        let attempt_id = string_cell(candidate_row, "id");
        let payment_intent_id = string_cell(candidate_row, "payment_intent_id");

        let intent_row = sqlx::query(
            r#"
            SELECT id, status
            FROM commerce_payment_intent
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .bind(&payment_intent_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner payment intent", error))?
        .ok_or_else(|| {
            CommerceServiceError::storage(
                "owner payment attempt references a missing or deleted payment intent",
            )
        })?;
        let intent_status = string_cell(&intent_row, "status");

        let locked_attempt = sqlx::query(
            r#"
            SELECT id, payment_intent_id, status, paid_at, out_trade_no
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
              AND payment_intent_id = CAST(? AS TEXT)
              AND (? IS NULL OR out_trade_no = CAST(? AS TEXT))
              AND deleted_at IS NULL
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .bind(&attempt_id)
        .bind(&payment_intent_id)
        .bind(out_trade_no)
        .bind(out_trade_no)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner payment attempt", error))?
        .ok_or_else(|| {
            CommerceServiceError::storage(
                "owner payment attempt changed identity during confirmation",
            )
        })?;
        let attempt_status = string_cell(&locked_attempt, "status");

        let intent_update = if payment_attempt_is_terminal_success(&intent_status) {
            None
        } else {
            crate::shared::ensure_payment_status_transition(
                &intent_status,
                CommercePaymentStatus::Succeeded.as_str(),
            )?;
            Some(
                sqlx::query(
                    r#"
            UPDATE commerce_payment_intent
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
              AND deleted_at IS NULL
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
                )
                .bind(CommercePaymentStatus::Succeeded.as_str())
                .bind(&confirmation_paid_at)
                .bind(tenant_id)
                .bind(organization_id)
                .bind(organization_id)
                .bind(owner_user_id)
                .bind(order_id)
                .bind(&payment_intent_id)
                .execute(&mut *tx)
                .await
                .map_err(|error| {
                    store_error("failed to mark owner payment intent succeeded", error)
                })?,
            )
        };
        ensure_confirmation_intent_update(
            intent_update
                .as_ref()
                .map_or(0, sqlx::sqlite::SqliteQueryResult::rows_affected),
            Some(intent_status.as_str()),
        )?;

        if payment_attempt_is_terminal_success(&attempt_status) {
            let paid_at = required_persisted_paid_at(&string_cell(&locked_attempt, "paid_at"))?;
            tx.commit().await.map_err(|error| {
                store_error("failed to commit idempotent payment confirmation", error)
            })?;
            return Ok(ConfirmOwnerOrderPaymentOutcome {
                tenant_id: tenant_id.to_owned(),
                organization_id: organization_id.map(str::to_owned),
                owner_user_id: owner_user_id.to_owned(),
                order_id: order_id.to_owned(),
                paid_at,
                replayed: true,
            });
        }

        crate::shared::ensure_payment_status_transition(
            &attempt_status,
            CommercePaymentStatus::Succeeded.as_str(),
        )?;

        let attempt_update = sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = ?, paid_at = COALESCE(NULLIF(paid_at, ''), ?), updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
              AND payment_intent_id = CAST(? AS TEXT)
              AND (? IS NULL OR out_trade_no = CAST(? AS TEXT))
              AND deleted_at IS NULL
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Succeeded.as_str())
        .bind(&confirmation_paid_at)
        .bind(&confirmation_paid_at)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .bind(&attempt_id)
        .bind(&payment_intent_id)
        .bind(out_trade_no)
        .bind(out_trade_no)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to mark owner payment attempt succeeded", error))?;

        let persisted_attempt = sqlx::query(
            r#"
            SELECT status, paid_at
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
              AND payment_intent_id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .bind(&attempt_id)
        .bind(&payment_intent_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| {
            store_error("failed to verify owner payment attempt confirmation", error)
        })?;

        let persisted_attempt_status = persisted_attempt
            .as_ref()
            .map(|row| string_cell(row, "status"));
        let replayed = resolve_confirmation_attempt_replayed(
            attempt_update.rows_affected(),
            persisted_attempt_status.as_deref(),
        )?;
        let paid_at = required_persisted_paid_at(
            &persisted_attempt
                .as_ref()
                .map(|row| string_cell(row, "paid_at"))
                .unwrap_or_default(),
        )?;

        tx.commit().await.map_err(|error| {
            store_error(
                "failed to commit owner order payment confirmation transaction",
                error,
            )
        })?;

        Ok(ConfirmOwnerOrderPaymentOutcome {
            tenant_id: tenant_id.to_owned(),
            organization_id: organization_id.map(str::to_owned),
            owner_user_id: owner_user_id.to_owned(),
            order_id: order_id.to_owned(),
            paid_at,
            replayed,
        })
    }
}

async fn load_owner_payment_outcome_by_idempotency_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PayOwnerOrderCommand,
    order_sn: &str,
    order_subject: Option<&str>,
) -> Result<Option<PayOwnerOrderOutcome>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT pa.id AS payment_attempt_id,
               pa.out_trade_no,
               CAST(pa.amount AS TEXT) AS amount,
               pa.payment_method,
               pa.provider_code,
               pa.channel_id,
               pa.status
        FROM commerce_payment_intent pi
        INNER JOIN commerce_payment_attempt pa
            ON pa.payment_intent_id = pi.id
           AND pa.tenant_id = pi.tenant_id
           AND pa.owner_user_id = pi.owner_user_id
           AND pa.order_id = pi.order_id
        WHERE pi.tenant_id = CAST(? AS TEXT)
          AND pi.order_id = CAST(? AS TEXT)
          AND pi.idempotency_key = CAST(? AS TEXT)
          AND ((pi.organization_id = CAST(? AS TEXT)) OR (pi.organization_id IS NULL AND ? IS NULL))
          AND pi.owner_user_id = CAST(? AS TEXT)
          AND pi.deleted_at IS NULL
        ORDER BY pa.created_at DESC, pa.id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(&command.idempotency_key)
    .bind(command.organization_id.as_deref())
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load owner payment idempotency replay", error))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let amount =
        CommerceMoney::new(&string_cell(&row, "amount")).map_err(CommerceServiceError::storage)?;
    let out_trade_no = string_cell(&row, "out_trade_no");
    let mut payment_params = owner_order_payment_params(
        &string_cell(&row, "provider_code"),
        order_sn,
        order_subject,
        &out_trade_no,
    );
    if let Some(channel_id) = optional_string_cell(&row, "channel_id") {
        payment_params.insert("channelId".to_owned(), channel_id);
    }

    let outcome = PayOwnerOrderOutcome {
        amount,
        order_id: command.order_id.clone(),
        out_trade_no,
        payment_id: string_cell(&row, "payment_attempt_id"),
        payment_method: string_cell(&row, "payment_method"),
        status: string_cell(&row, "status"),
        payment_params,
    };
    ensure_owner_payment_idempotency_replay_matches(command, &outcome)?;
    Ok(Some(outcome))
}

async fn load_reusable_owner_payment_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    command: &PayOwnerOrderCommand,
    order_sn: &str,
    order_subject: Option<&str>,
) -> Result<Option<PayOwnerOrderOutcome>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT pa.id, pa.out_trade_no, pa.amount, pa.payment_method, pa.provider_code,
               pa.channel_id, pa.status
        FROM commerce_payment_attempt pa
        INNER JOIN commerce_order o
            ON o.id = pa.order_id
           AND o.tenant_id = pa.tenant_id
           AND o.owner_user_id = pa.owner_user_id
        WHERE pa.tenant_id = CAST(? AS TEXT)
          AND pa.owner_user_id = CAST(? AS TEXT)
          AND pa.order_id = CAST(? AS TEXT)
          AND pa.payment_method = CAST(? AS TEXT)
          AND LOWER(COALESCE(pa.status, '')) IN ('created', 'pending', 'processing')
        ORDER BY pa.created_at DESC, pa.id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.owner_user_id)
    .bind(&command.order_id)
    .bind(&command.payment_method)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load reusable owner payment", error))?;

    let Some(row) = row else {
        return Ok(None);
    };

    let amount =
        CommerceMoney::new(&string_cell(&row, "amount")).map_err(CommerceServiceError::storage)?;
    let out_trade_no = string_cell(&row, "out_trade_no");
    let mut payment_params = owner_order_payment_params(
        &string_cell(&row, "provider_code"),
        order_sn,
        order_subject,
        &out_trade_no,
    );
    if let Some(channel_id) = optional_string_cell(&row, "channel_id") {
        payment_params.insert("channelId".to_owned(), channel_id);
    }

    Ok(Some(PayOwnerOrderOutcome {
        amount,
        order_id: command.order_id.clone(),
        out_trade_no,
        payment_id: string_cell(&row, "id"),
        payment_method: string_cell(&row, "payment_method"),
        status: string_cell(&row, "status"),
        payment_params,
    }))
}

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{message}: {error}"))
}

fn string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> String {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}
