use sdkwork_contract_service::{
    CommerceMoney, CommercePaymentStatus, CommerceServiceError,
};
use sdkwork_payment_service::{
    CancelOrderPaymentsCommand, ConfirmOwnerOrderPaymentOutcome, PayOwnerOrderCommand,
    PayOwnerOrderOutcome,
};
use sqlx::{Postgres, PgPool, Row, Transaction};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::order_reference::order_status_is_payable;
use crate::owner_payment_params::owner_order_payment_params;
use crate::shared::{payment_attempt_is_terminal_success, stable_storage_id};

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
        let now = current_command_timestamp();
        // C1/C2 修复：取消操作必须在单事务内执行，且只能取消未终结状态的支付意图/尝试，
        // 严禁覆盖 succeeded/refunded/closed 等终态，避免"已收款但订单显示已取消"的资金事故。
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin cancel owner order payment transaction", error))?;

        let affected_intent = sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = $1, updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close order payment intents", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = $1, updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to close order payment attempts", error))?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit cancel owner order payment transaction", error)
        })?;

        // 事务提交后校验是否存在已成功但未取消的支付意图，若有则上报冲突，
        // 避免调用方误以为取消成功而实际仍存在有效支付。
        if affected_intent.rows_affected() == 0 {
            let existing = sqlx::query(
                r#"
                SELECT 1
                FROM commerce_payment_intent
                WHERE tenant_id = CAST($1 AS TEXT)
                  AND owner_user_id = CAST($2 AS TEXT)
                  AND order_id = CAST($3 AS TEXT)
                  AND LOWER(COALESCE(status, '')) NOT IN ('created', 'pending', 'processing', 'canceled')
                LIMIT 1
                "#,
            )
            .bind(&command.tenant_id)
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

        if let Some(existing) =
            load_owner_payment_outcome_by_idempotency_in_tx(&mut tx, &command, &order_sn, order_subject.as_deref()).await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit idempotent owner payment replay", error))?;
            return Ok(existing);
        }

        if let Some(existing) =
            load_reusable_owner_payment_in_tx(&mut tx, &command, &order_sn, order_subject.as_deref()).await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit reusable owner payment", error))?;
            return Ok(existing);
        }

        let method = load_owner_payment_method(&mut tx, &command).await?;
        let now = current_command_timestamp();
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
        let out_trade_no = format!("OT-{}-{}", order_sn, &command.idempotency_key[..command.idempotency_key.len().min(24)]);

        sqlx::query(
            r#"
            INSERT INTO commerce_payment_intent
                (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                 payment_method, provider_code, amount, currency_code, status, request_no,
                 idempotency_key, created_at, updated_at)
            VALUES
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, 'CNY', $10, $11, $12, $13, $14)
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

        if let Some(existing) =
            load_owner_payment_outcome_by_idempotency_in_tx(&mut tx, &command, &order_sn, order_subject.as_deref()).await?
        {
            tx.commit()
                .await
                .map_err(|error| store_error("failed to commit idempotent owner payment replay", error))?;
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
                ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, 'CNY', $11, $12, $13, $14, $15, NULL, $16)
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
        tenant_id: &str,
        organization_id: Option<&str>,
        owner_user_id: &str,
        order_id: &str,
    ) -> Result<ConfirmOwnerOrderPaymentOutcome, CommerceServiceError> {
        let paid_at = current_command_timestamp();
        let mut tx = self.pool.begin().await.map_err(|error| {
            store_error("failed to begin owner order payment confirmation transaction", error)
        })?;

        let attempt_row = sqlx::query(
            r#"
            SELECT id, status
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST($1 AS TEXT)
              AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $3::text IS NULL))
              AND owner_user_id = CAST($4 AS TEXT)
              AND order_id = CAST($5 AS TEXT)
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load owner order payment attempt", error))?;

        let Some(attempt_row) = attempt_row else {
            return Err(CommerceServiceError::not_found(
                "owner order payment attempt was not found",
            ));
        };

        let attempt_status = string_cell(&attempt_row, "status");
        if !payment_attempt_is_terminal_success(&attempt_status) {
            crate::shared::ensure_payment_status_transition(&attempt_status, CommercePaymentStatus::Succeeded.as_str())?;
        }
        if payment_attempt_is_terminal_success(&attempt_status) {
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

        sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = $1, updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND ((organization_id = CAST($4 AS TEXT)) OR (organization_id IS NULL AND $5::text IS NULL))
              AND owner_user_id = CAST($6 AS TEXT)
              AND order_id = CAST($7 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Succeeded.as_str())
        .bind(&paid_at)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to mark owner payment intent succeeded", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = $1, paid_at = $2, updated_at = $3
            WHERE tenant_id = CAST($4 AS TEXT)
              AND ((organization_id = CAST($5 AS TEXT)) OR (organization_id IS NULL AND $6::text IS NULL))
              AND owner_user_id = CAST($7 AS TEXT)
              AND order_id = CAST($8 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Succeeded.as_str())
        .bind(&paid_at)
        .bind(&paid_at)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(owner_user_id)
        .bind(order_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to mark owner payment attempt succeeded", error))?;

        tx.commit().await.map_err(|error| {
            store_error("failed to commit owner order payment confirmation transaction", error)
        })?;

        Ok(ConfirmOwnerOrderPaymentOutcome {
            tenant_id: tenant_id.to_owned(),
            organization_id: organization_id.map(str::to_owned),
            owner_user_id: owner_user_id.to_owned(),
            order_id: order_id.to_owned(),
            paid_at,
            replayed: false,
        })
    }

}

struct OwnerPaymentMethod {
    method_key: String,
    provider_code: String,
}

fn current_command_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format!("{seconds}")
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
