#![allow(clippy::too_many_arguments)]

use sdkwork_contract_service::{
    CommerceMoney, CommercePaymentStatus, CommerceServiceError,
};
use sdkwork_order_service::{OrderOwnerDetailQuery, PayOwnerOrderCommand};
use sdkwork_order_repository_sqlx::SqliteCommerceOrderStore;
use std::sync::Arc;
use sdkwork_payment_service::{
    CancelOwnerPaymentIntentCommand, CreateOwnerPaymentAttemptCommand,
    CreateOwnerPaymentAttemptOutcome, CreateOwnerPaymentIntentCommand, PaymentIntentDetailQuery,
    PaymentIntentView,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};



#[derive(Debug, Clone)]
struct ResolvedPaymentMethod {
    method_key: String,
    provider_code: String,
}

#[derive(Debug, Clone)]
pub struct SqliteCommercePaymentIntentStore {
    pool: SqlitePool,
    order_store: Arc<SqliteCommerceOrderStore>,
}

impl SqliteCommercePaymentIntentStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self {
            pool: pool.clone(),
            order_store: Arc::new(SqliteCommerceOrderStore::new(pool)),
        }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

impl SqliteCommercePaymentIntentStore {
    pub async fn create_owner_payment_intent(
        &self,
        command: CreateOwnerPaymentIntentCommand,
    ) -> Result<PaymentIntentView, CommerceServiceError> {
        if let Some(existing) = self
            .find_owner_payment_intent_by_idempotency(&command)
            .await?
        {
            return Ok(existing);
        }

        let detail_query = OrderOwnerDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.order_id,
        )?;
        let Some(detail) = self.order_store.retrieve_owner_order(detail_query).await? else {
            return Err(CommerceServiceError::not_found("order was not found"));
        };
        if !order_status_is_payable(&detail.summary.status) {
            return Err(CommerceServiceError::conflict(
                "order is not pending payment",
            ));
        }

        let mut tx =
            self.pool().begin_with("BEGIN IMMEDIATE").await.map_err(|error| {
                store_error("failed to begin payment intent transaction", error)
            })?;
        let method = load_payment_method_for_intent(&mut tx, &command).await?;
        let now = current_timestamp_string();
        let payment_intent_id = payment_intent_id(&command);
        let status = CommercePaymentStatus::Pending.as_str();

        // C5 修复：使用 ON CONFLICT (id) DO NOTHING 原子化幂等插入，消除 TOCTOU 竞态。
        let inserted = sqlx::query(
            r#"
            INSERT INTO commerce_payment_intent
                (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no,
                 payment_method, provider_code, amount, currency_code, status, request_no,
                 idempotency_key, created_at, updated_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?, 'CNY', ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&payment_intent_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .bind(&command.request_no)
        .bind(&method.method_key)
        .bind(&method.provider_code)
        .bind(detail.summary.total_amount.as_str())
        .bind(status)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert payment intent", error))?;

        if inserted.rows_affected() == 0 {
            tx.commit().await.map_err(|error| {
                store_error("failed to commit payment intent transaction after conflict", error)
            })?;
            if let Some(existing) = self
                .find_owner_payment_intent_by_idempotency(&command)
                .await?
            {
                return Ok(existing);
            }
            return Err(CommerceServiceError::conflict(
                "payment intent idempotency conflict: existing record not found",
            ));
        }

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit payment intent transaction", error))?;

        Ok(PaymentIntentView {
            payment_intent_id,
            order_id: command.order_id,
            payment_intent_no: command.request_no,
            payment_method: method.method_key,
            provider_code: method.provider_code,
            amount: detail.summary.total_amount,
            currency_code: "CNY".to_owned(),
            status: status.to_owned(),
        })
    }

    pub async fn retrieve_owner_payment_intent(
        &self,
        query: PaymentIntentDetailQuery,
    ) -> Result<Option<PaymentIntentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, order_id, payment_intent_no, payment_method, provider_code,
                   CAST(amount AS TEXT) AS amount, currency_code, status
            FROM commerce_payment_intent
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND owner_user_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.payment_intent_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve payment intent", error))?;

        row.map(map_payment_intent_row).transpose()
    }

    pub async fn cancel_owner_payment_intent(
        &self,
        command: CancelOwnerPaymentIntentCommand,
    ) -> Result<PaymentIntentView, CommerceServiceError> {
        let query = PaymentIntentDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.payment_intent_id,
        )?;
        let Some(intent) = self.retrieve_owner_payment_intent(query).await? else {
            return Err(CommerceServiceError::not_found(
                "payment intent was not found",
            ));
        };
        if matches!(
            intent.status.to_ascii_lowercase().as_str(),
            "succeeded" | "closed" | "canceled" | "cancelled"
        ) {
            return Err(CommerceServiceError::conflict(
                "payment intent is not cancelable",
            ));
        }

        let now = current_timestamp_string();
        let canceled = CommercePaymentStatus::Canceled.as_str();
        sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND owner_user_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
            "#,
        )
        .bind(canceled)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.payment_intent_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to cancel payment intent", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND owner_user_id = CAST(? AS TEXT)
              AND payment_intent_id = CAST(? AS TEXT)
            "#,
        )
        .bind(canceled)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.payment_intent_id)
        .execute(self.pool())
        .await
        .map_err(|error| store_error("failed to cancel payment attempts", error))?;

        Ok(PaymentIntentView {
            status: canceled.to_owned(),
            ..intent
        })
    }

    pub async fn create_owner_payment_attempt(
        &self,
        command: CreateOwnerPaymentAttemptCommand,
    ) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
        if let Some(existing) = self
            .find_owner_payment_attempt_by_idempotency(&command)
            .await?
        {
            return Ok(existing);
        }

        let intent_query = PaymentIntentDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.payment_intent_id,
        )?;
        let Some(intent) = self.retrieve_owner_payment_intent(intent_query).await? else {
            return Err(CommerceServiceError::not_found(
                "payment intent was not found",
            ));
        };
        if matches!(
            intent.status.to_ascii_lowercase().as_str(),
            "succeeded" | "closed" | "canceled" | "cancelled"
        ) {
            return Err(CommerceServiceError::conflict(
                "payment intent is not attemptable",
            ));
        }

        if let Some(existing) = load_reusable_payment_attempt(self.pool(), &command).await? {
            return Ok(existing);
        }

        let detail_query = OrderOwnerDetailQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &intent.order_id,
        )?;
        let Some(detail) = self.order_store.retrieve_owner_order(detail_query).await? else {
            return Err(CommerceServiceError::not_found("order was not found"));
        };

        let mut tx =
            self.pool().begin().await.map_err(|error| {
                store_error("failed to begin payment attempt transaction", error)
            })?;
        let now = current_timestamp_string();
        let attempt_id = payment_attempt_id(&command);
        let out_trade_no = format!(
            "OT-{}-{}",
            detail.summary.order_sn,
            command.idempotency_key.replace('-', "")
        );
        let pending = CommercePaymentStatus::Pending.as_str();

        sqlx::query(
            r#"
            INSERT INTO commerce_payment_attempt
                (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
                 payment_method, provider_code, out_trade_no, amount, currency_code, status,
                 callback_payload, created_at, paid_at, updated_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, '{}', ?, NULL, ?)
            "#,
        )
        .bind(&attempt_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.payment_intent_id)
        .bind(&intent.order_id)
        .bind(&intent.payment_method)
        .bind(&intent.provider_code)
        .bind(&out_trade_no)
        .bind(intent.amount.as_str())
        .bind(&intent.currency_code)
        .bind(pending)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert payment attempt", error))?;

        sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = ?, updated_at = ?
            WHERE tenant_id = CAST(? AS TEXT)
              AND owner_user_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
            "#,
        )
        .bind(pending)
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.payment_intent_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to update payment intent status", error))?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit payment attempt transaction", error))?;

        Ok(CreateOwnerPaymentAttemptOutcome {
            attempt_id,
            payment_intent_id: command.payment_intent_id,
            order_id: intent.order_id,
            out_trade_no,
            amount: intent.amount,
            payment_method: intent.payment_method,
            status: pending.to_owned(),
        })
    }

    async fn find_owner_payment_intent_by_idempotency(
        &self,
        command: &CreateOwnerPaymentIntentCommand,
    ) -> Result<Option<PaymentIntentView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, order_id, payment_intent_no, payment_method, provider_code,
                   CAST(amount AS TEXT) AS amount, currency_code, status
            FROM commerce_payment_intent
            WHERE tenant_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND idempotency_key = CAST(? AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.order_id)
        .bind(&command.idempotency_key)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load payment intent idempotency replay", error))?;

        row.map(map_payment_intent_row).transpose()
    }

    async fn find_owner_payment_attempt_by_idempotency(
        &self,
        command: &CreateOwnerPaymentAttemptCommand,
    ) -> Result<Option<CreateOwnerPaymentAttemptOutcome>, CommerceServiceError> {
        let attempt_id = payment_attempt_id(command);
        let row = sqlx::query(
            r#"
            SELECT id, payment_intent_id, order_id, out_trade_no, CAST(amount AS TEXT) AS amount,
                   payment_method, status
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND owner_user_id = CAST(? AS TEXT)
              AND id = CAST(? AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&attempt_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load payment attempt idempotency replay", error))?;

        row.map(map_payment_attempt_row).transpose()
    }
}

async fn load_payment_method_for_intent(
    tx: &mut Transaction<'_, Sqlite>,
    command: &CreateOwnerPaymentIntentCommand,
) -> Result<ResolvedPaymentMethod, CommerceServiceError> {
    let pay_command = PayOwnerOrderCommand::new(
        &command.tenant_id,
        command.organization_id.as_deref(),
        &command.owner_user_id,
        &command.order_id,
        &command.payment_method,
    )?;
    let row = sqlx::query(
        r#"
        SELECT method_key, provider_code
        FROM commerce_payment_method
        WHERE method_key = ?
          AND status = 'active'
          AND (
                (tenant_id = CAST(? AS TEXT) AND organization_id = CAST(? AS TEXT))
             OR (tenant_id = CAST(? AS TEXT) AND organization_id IS NULL)
             OR (tenant_id = '100001' AND (organization_id = '0' OR organization_id IS NULL))
          )
        ORDER BY CASE WHEN tenant_id = CAST(? AS TEXT) THEN 0 ELSE 1 END, sort_order ASC
        LIMIT 1
        "#,
    )
    .bind(&pay_command.payment_method)
    .bind(&pay_command.tenant_id)
    .bind(pay_command.organization_id.as_deref())
    .bind(&pay_command.tenant_id)
    .bind(&pay_command.tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load payment method for intent", error))?;

    row.map(|row| ResolvedPaymentMethod {
        method_key: string_cell(&row, "method_key"),
        provider_code: string_cell(&row, "provider_code"),
    })
    .ok_or_else(|| CommerceServiceError::conflict("payment method is unavailable"))
}

async fn load_reusable_payment_attempt(
    pool: &sqlx::SqlitePool,
    command: &CreateOwnerPaymentAttemptCommand,
) -> Result<Option<CreateOwnerPaymentAttemptOutcome>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, payment_intent_id, order_id, out_trade_no, CAST(amount AS TEXT) AS amount,
               payment_method, status
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST(? AS TEXT)
          AND owner_user_id = CAST(? AS TEXT)
          AND payment_intent_id = CAST(? AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.owner_user_id)
    .bind(&command.payment_intent_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load reusable payment attempt", error))?;

    row.map(map_payment_attempt_row).transpose()
}

fn map_payment_intent_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<PaymentIntentView, CommerceServiceError> {
    Ok(PaymentIntentView {
        payment_intent_id: string_cell(&row, "id"),
        order_id: string_cell(&row, "order_id"),
        payment_intent_no: string_cell(&row, "payment_intent_no"),
        payment_method: string_cell(&row, "payment_method"),
        provider_code: string_cell(&row, "provider_code"),
        amount: CommerceMoney::new(&string_cell(&row, "amount"))
            .map_err(CommerceServiceError::storage)?,
        currency_code: string_cell(&row, "currency_code"),
        status: string_cell(&row, "status"),
    })
}

fn map_payment_attempt_row(
    row: sqlx::sqlite::SqliteRow,
) -> Result<CreateOwnerPaymentAttemptOutcome, CommerceServiceError> {
    Ok(CreateOwnerPaymentAttemptOutcome {
        attempt_id: string_cell(&row, "id"),
        payment_intent_id: string_cell(&row, "payment_intent_id"),
        order_id: string_cell(&row, "order_id"),
        out_trade_no: string_cell(&row, "out_trade_no"),
        amount: CommerceMoney::new(&string_cell(&row, "amount"))
            .map_err(CommerceServiceError::storage)?,
        payment_method: string_cell(&row, "payment_method"),
        status: string_cell(&row, "status"),
    })
}

fn payment_intent_id(command: &CreateOwnerPaymentIntentCommand) -> String {
    stable_storage_id(&[
        "payment-intent",
        &command.tenant_id,
        &command.order_id,
        &command.idempotency_key,
    ])
}

fn payment_attempt_id(command: &CreateOwnerPaymentAttemptCommand) -> String {
    stable_storage_id(&[
        "payment-attempt",
        &command.tenant_id,
        &command.payment_intent_id,
        &command.idempotency_key,
    ])
}

fn order_status_is_payable(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "pending_payment" | "pending" | "created" | "unpaid"
    )
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

fn store_error(message: &str, error: impl std::fmt::Display) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{message}: {error}"))
}

fn current_timestamp_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format!("{seconds}")
}

fn optional_string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}
