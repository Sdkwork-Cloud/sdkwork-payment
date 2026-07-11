use sdkwork_contract_service::{CommerceMoney, CommercePaymentStatus, CommerceServiceError};
use sdkwork_payment_service::{
    CancelOrderPaymentsCommand, ConfirmOwnerOrderPaymentOutcome, PayOwnerOrderCommand,
    PayOwnerOrderOutcome, OrderPaymentSettlementAttempt,
};
use sqlx::{PgPool, Postgres, Row, Transaction};

use crate::order_reference::order_status_is_payable;
use crate::owner_payment_params::owner_order_payment_params;
use crate::shared::{
    current_timestamp_string, ensure_confirmation_intent_update,
    payment_attempt_is_terminal_success, required_persisted_paid_at,
    resolve_confirmation_attempt_replayed, stable_storage_id,
};

const LOAD_OWNER_ORDER_FOR_CONFIRMATION: &str = r#"
SELECT id
FROM commerce_order
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
  AND owner_user_id = CAST($4 AS TEXT)
  AND id = CAST($5 AS TEXT)
  AND deleted_at IS NULL
FOR UPDATE
"#;

const FIND_OWNER_PAYMENT_ATTEMPT_CANDIDATES: &str = r#"
SELECT id, payment_intent_id, status,
       to_char(paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS paid_at,
       out_trade_no
FROM commerce_payment_attempt
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
  AND owner_user_id = CAST($4 AS TEXT)
  AND order_id = CAST($5 AS TEXT)
  AND ($6::text IS NULL OR id = CAST($6 AS TEXT))
  AND ($7::text IS NULL OR out_trade_no = CAST($7 AS TEXT))
  AND deleted_at IS NULL
ORDER BY id
LIMIT 2
"#;

const LOAD_OWNER_PAYMENT_INTENT_FOR_CONFIRMATION: &str = r#"
SELECT id, status
FROM commerce_payment_intent
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
  AND owner_user_id = CAST($4 AS TEXT)
  AND order_id = CAST($5 AS TEXT)
  AND id = CAST($6 AS TEXT)
  AND deleted_at IS NULL
FOR UPDATE
"#;

const LOAD_OWNER_PAYMENT_ATTEMPT_FOR_CONFIRMATION: &str = r#"
SELECT id, payment_intent_id, status,
       to_char(paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS paid_at,
       out_trade_no
FROM commerce_payment_attempt
WHERE tenant_id = CAST($1 AS TEXT)
  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
  AND owner_user_id = CAST($4 AS TEXT)
  AND order_id = CAST($5 AS TEXT)
  AND id = CAST($6 AS TEXT)
  AND payment_intent_id = CAST($7 AS TEXT)
  AND ($8::text IS NULL OR out_trade_no = CAST($8 AS TEXT))
  AND deleted_at IS NULL
FOR UPDATE
"#;

#[derive(Debug, Clone)]
pub struct PostgresCommerceOwnerOrderPaymentStore {
    pool: PgPool,
}

impl PostgresCommerceOwnerOrderPaymentStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub async fn cancel_order_payments(
        &self,
        command: CancelOrderPaymentsCommand,
    ) -> Result<(), CommerceServiceError> {
        let now = current_timestamp_string();
        // C1/C2 修复：取消操作必须在单事务内执行，且只能取消未终结状态的支付意图/尝试，
        // 严禁覆盖 succeeded/refunded/closed 等终态，避免"已收款但订单显示已取消"的资金事故。
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error(
                "failed to begin cancel owner order payment transaction",
                error,
            )
        })?;

        sqlx::query(
            r#"
            SELECT id
            FROM commerce_order
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
              AND deleted_at IS NULL
            FOR UPDATE
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.organization_id)
        .bind(&command.organization_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to lock owner order for payment cancellation", error))?;

        sqlx::query(
            r#"
            SELECT id
            FROM commerce_payment_intent
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
              AND deleted_at IS NULL
            ORDER BY id
            FOR UPDATE
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
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
              AND deleted_at IS NULL
            ORDER BY id
            FOR UPDATE
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
            SET status = $1, updated_at = $2::timestamptz
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $5::text IS NULL))
              AND owner_user_id = CAST($6 AS TEXT)
              AND order_id = CAST($7 AS TEXT)
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
            SET status = $1, updated_at = $2::timestamptz
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $5::text IS NULL))
              AND owner_user_id = CAST($6 AS TEXT)
              AND order_id = CAST($7 AS TEXT)
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

        // 事务提交后校验是否存在已成功但未取消的支付意图，若有则上报冲突，
        // 避免调用方误以为取消成功而实际仍存在有效支付。
        if affected_intent.rows_affected() == 0 {
            let existing = sqlx::query(
                r#"
                SELECT 1
                FROM commerce_payment_intent
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
                  AND owner_user_id = CAST($4 AS TEXT)
                  AND order_id = CAST($5 AS TEXT)
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
            .fetch_optional(self.pool())
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
        let mut tx = self.pool.begin().await.map_err(|error| {
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
            WHERE o.id = CAST($1 AS TEXT)
              AND o.tenant_id = CAST($2 AS TEXT)
              AND ((o.organization_id = CAST($3 AS TEXT)) OR (o.organization_id IS NULL AND $4 IS NULL))
              AND o.owner_user_id = CAST($5 AS TEXT)
            FOR UPDATE
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

        let method = load_owner_payment_method(&mut tx, &command).await?;
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
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, 'CNY', $10, $11, $12, $13::timestamptz, $14::timestamptz)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&payment_intent_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .bind(format!("PAY-{}", order_sn))
        .bind(&method.method_key)
        .bind(&method.provider_code)
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
                 payment_method, provider_code, out_trade_no, amount, currency_code, status,
                 callback_payload, request_no, idempotency_key, created_at, paid_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, 'CNY', $11, $12, $13, $14, $15::timestamptz, NULL, $16::timestamptz)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&payment_attempt_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&payment_intent_id)
        .bind(&command.order_id)
        .bind(&method.method_key)
        .bind(&method.provider_code)
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
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error(
                "failed to begin owner order payment confirmation transaction",
                error,
            )
        })?;

        let order_row = sqlx::query(LOAD_OWNER_ORDER_FOR_CONFIRMATION)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(order_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| store_error("failed to lock owner order for payment confirmation", error))?;
        if order_row.is_none() {
            return Err(CommerceServiceError::not_found(
                "owner order was not found for payment confirmation",
            ));
        }

        let candidate_rows = sqlx::query(FIND_OWNER_PAYMENT_ATTEMPT_CANDIDATES)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(order_id)
            .bind(payment_attempt_id)
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

        let intent_row = sqlx::query(LOAD_OWNER_PAYMENT_INTENT_FOR_CONFIRMATION)
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

        let locked_attempt = sqlx::query(LOAD_OWNER_PAYMENT_ATTEMPT_FOR_CONFIRMATION)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(organization_id)
            .bind(owner_user_id)
            .bind(order_id)
            .bind(&attempt_id)
            .bind(&payment_intent_id)
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
            SET status = $1, updated_at = $2::timestamptz
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $5::text IS NULL))
              AND owner_user_id = CAST($6 AS TEXT)
              AND order_id = CAST($7 AS TEXT)
              AND id = CAST($8 AS TEXT)
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
        .map_err(|error| store_error("failed to mark owner payment intent succeeded", error))?,
            )
        };
        ensure_confirmation_intent_update(
            intent_update
                .as_ref()
                .map_or(0, sqlx::postgres::PgQueryResult::rows_affected),
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
            SET status = $1,
                paid_at = COALESCE(paid_at, $2::timestamptz),
                updated_at = $3::timestamptz
            WHERE tenant_id = CAST($4 AS TEXT)
              AND ((organization_id = CAST($5 AS TEXT)) OR (organization_id IS NULL AND $6::text IS NULL))
              AND owner_user_id = CAST($7 AS TEXT)
              AND order_id = CAST($8 AS TEXT)
              AND id = CAST($9 AS TEXT)
              AND payment_intent_id = CAST($10 AS TEXT)
              AND ($11::text IS NULL OR out_trade_no = CAST($11 AS TEXT))
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
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to mark owner payment attempt succeeded", error))?;

        let persisted_attempt = sqlx::query(
            r#"
            SELECT status,
                   to_char(paid_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS.MS"Z"') AS paid_at
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
              AND id = CAST($6 AS TEXT)
              AND payment_intent_id = CAST($7 AS TEXT)
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
        .map_err(|error| store_error("failed to verify owner payment attempt confirmation", error))?;

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

struct OwnerPaymentMethod {
    method_key: String,
    provider_code: String,
}

async fn load_owner_payment_method(
    tx: &mut Transaction<'_, Postgres>,
    command: &PayOwnerOrderCommand,
) -> Result<OwnerPaymentMethod, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT method_key, provider_code
        FROM commerce_payment_method
        WHERE method_key = $1
          AND status = 'active'
          AND (
                (tenant_id = CAST($2 AS TEXT) AND organization_id = CAST($3 AS TEXT))
             OR (tenant_id = CAST($4 AS TEXT) AND organization_id IS NULL)
          )
        ORDER BY CASE WHEN tenant_id = CAST($5 AS TEXT) THEN 0 ELSE 1 END, sort_order ASC
        LIMIT 1
        "#,
    )
    .bind(&command.payment_method)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.tenant_id)
    .bind(&command.tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load owner payment method", error))?;

    row.map(|row| OwnerPaymentMethod {
        method_key: string_cell(&row, "method_key"),
        provider_code: string_cell(&row, "provider_code"),
    })
    .ok_or_else(|| CommerceServiceError::conflict("payment method is unavailable"))
}

async fn load_owner_payment_outcome_by_idempotency_in_tx(
    tx: &mut Transaction<'_, Postgres>,
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
               pa.status
        FROM commerce_payment_intent pi
        INNER JOIN commerce_payment_attempt pa
            ON pa.payment_intent_id = pi.id
           AND pa.tenant_id = pi.tenant_id
           AND pa.owner_user_id = pi.owner_user_id
           AND pa.order_id = pi.order_id
        WHERE pi.tenant_id = CAST($1 AS TEXT)
          AND pi.order_id = CAST($2 AS TEXT)
          AND pi.idempotency_key = CAST($3 AS TEXT)
          AND pi.owner_user_id = CAST($4 AS TEXT)
          AND pi.deleted_at IS NULL
        ORDER BY pa.created_at DESC, pa.id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(&command.idempotency_key)
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
    let payment_params = owner_order_payment_params(
        &string_cell(&row, "provider_code"),
        order_sn,
        order_subject,
        &out_trade_no,
    );

    Ok(Some(PayOwnerOrderOutcome {
        amount,
        order_id: command.order_id.clone(),
        out_trade_no,
        payment_id: string_cell(&row, "payment_attempt_id"),
        payment_method: string_cell(&row, "payment_method"),
        status: string_cell(&row, "status"),
        payment_params,
    }))
}

async fn load_reusable_owner_payment_in_tx(
    tx: &mut Transaction<'_, Postgres>,
    command: &PayOwnerOrderCommand,
    order_sn: &str,
    order_subject: Option<&str>,
) -> Result<Option<PayOwnerOrderOutcome>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT pa.id, pa.out_trade_no, pa.amount, pa.payment_method, pa.provider_code, pa.status
        FROM commerce_payment_attempt pa
        INNER JOIN commerce_order o
            ON o.id = pa.order_id
           AND o.tenant_id = pa.tenant_id
           AND o.owner_user_id = pa.owner_user_id
        WHERE pa.tenant_id = CAST($1 AS TEXT)
          AND pa.owner_user_id = CAST($2 AS TEXT)
          AND pa.order_id = CAST($3 AS TEXT)
          AND pa.payment_method = CAST($4 AS TEXT)
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
    let payment_params = owner_order_payment_params(
        &string_cell(&row, "provider_code"),
        order_sn,
        order_subject,
        &out_trade_no,
    );

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

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .unwrap_or_default()
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column)
        .ok()
        .flatten()
        .filter(|value| !value.trim().is_empty())
}

#[cfg(test)]
mod tests {
    use super::LOAD_OWNER_PAYMENT_ATTEMPT_FOR_CONFIRMATION;

    #[test]
    fn confirmation_attempt_lock_query_locks_the_latest_attempt() {
        let query = LOAD_OWNER_PAYMENT_ATTEMPT_FOR_CONFIRMATION
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(query.contains("SELECT id, payment_intent_id, status, paid_at"));
        assert!(query.ends_with("LIMIT 1 FOR UPDATE"));
    }
}
