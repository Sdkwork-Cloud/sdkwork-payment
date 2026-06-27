#![allow(clippy::too_many_arguments)]

use sdkwork_commerce_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_commerce_order_service::OrderOwnerDetailQuery;
use sdkwork_commerce_order_repository_sqlx::SqliteCommerceOrderStore;
use std::sync::Arc;
use sdkwork_commerce_payment_service::{
    CreateOwnerRefundCommand, RefundDetailQuery, RefundListQuery, RefundView,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::shared::{
    current_timestamp_string, money_to_minor_cents, resolve_refund_amount,
    stable_storage_id, store_error, validate_refund_bounds,
};



#[derive(Debug, Clone)]
pub struct SqliteCommerceRefundStore {
    pool: SqlitePool,
    order_store: Arc<SqliteCommerceOrderStore>,
}

impl SqliteCommerceRefundStore {
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

impl SqliteCommerceRefundStore {
    pub async fn create_owner_refund(
        &self,
        command: CreateOwnerRefundCommand,
    ) -> Result<RefundView, CommerceServiceError> {
        if let Some(existing) = self.find_refund_by_idempotency(&command).await? {
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
        // C3 修复：退款资格必须用 OR 语义——订单状态非 paid 或 无支付时间 任一不满足即拒绝。
        if detail.summary.status != "paid" || detail.summary.pay_time.is_none() {
            return Err(CommerceServiceError::conflict(
                "order is not eligible for refund",
            ));
        }

        let payment_attempt_id = match command.payment_attempt_id.as_deref() {
            Some(value) => value.to_owned(),
            None => self
                .find_latest_succeeded_payment_attempt(&command)
                .await?
                .ok_or_else(|| CommerceServiceError::not_found("payment attempt was not found"))?,
        };

        let refund_amount = resolve_refund_amount(&command, &detail.summary.total_amount)?;
        let total_minor = money_to_minor_cents(detail.summary.total_amount.as_str())?;
        let refund_minor = money_to_minor_cents(&refund_amount)?;
        validate_refund_bounds(refund_minor, total_minor)?;
        let already_refunded_minor = self.sum_refunded_amount(&command).await?;
        if refund_minor > total_minor.saturating_sub(already_refunded_minor) {
            return Err(CommerceServiceError::conflict(
                "refund amount exceeds remaining refundable amount",
            ));
        }

        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|error| store_error("failed to begin refund transaction", error))?;
        let now = current_timestamp_string();
        let refund_id = refund_id(&command);
        let refund_no = format!("RF-{}", command.request_no);

        sqlx::query(
            r#"
            INSERT INTO commerce_refund
                (id, tenant_id, organization_id, order_id, payment_attempt_id, refund_no,
                 amount, currency_code, status, refund_reason_code, requested_by_type,
                 requested_by, request_no, idempotency_key, created_at, updated_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, 'CNY', 'submitted', ?, 'buyer', ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&refund_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .bind(&payment_attempt_id)
        .bind(&refund_no)
        .bind(&refund_amount)
        .bind(command.reason_code.as_deref())
        .bind(&command.owner_user_id)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert refund", error))?;

        insert_refund_event(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &refund_id,
            "created",
            "submitted",
            &command.request_no,
            &command.idempotency_key,
            &now,
        )
        .await?;

        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit refund transaction", error))?;

        Ok(RefundView {
            refund_id,
            refund_no,
            order_id: command.order_id,
            payment_attempt_id,
            amount: CommerceMoney::new(&refund_amount).map_err(CommerceServiceError::storage)?,
            currency_code: "CNY".to_owned(),
            status: "submitted".to_owned(),
            reason_code: command.reason_code,
        })
    }

    pub async fn list_owner_refunds(
        &self,
        query: RefundListQuery,
    ) -> Result<Vec<RefundView>, CommerceServiceError> {
        let mut sql = String::from(
            r#"
            SELECT r.id, r.refund_no, r.order_id, r.payment_attempt_id,
                   CAST(r.amount AS TEXT) AS amount, r.currency_code, r.status, r.refund_reason_code
            FROM commerce_refund r
            INNER JOIN commerce_order o
                ON o.tenant_id = r.tenant_id
               AND o.id = r.order_id
            WHERE r.tenant_id = CAST(? AS TEXT)
              AND ((r.organization_id = CAST(? AS TEXT)) OR (r.organization_id IS NULL AND ? IS NULL))
              AND o.owner_user_id = CAST(? AS TEXT)
            "#,
        );
        if query.status.is_some() {
            sql.push_str(" AND r.status = CAST(? AS TEXT)");
        }
        sql.push_str(" ORDER BY r.created_at DESC, r.id DESC");

        let mut db_query = sqlx::query(&sql)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id);
        if let Some(status) = query.status.as_deref() {
            db_query = db_query.bind(status);
        }

        let rows = db_query
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list owner refunds", error))?;

        rows.into_iter().map(map_refund_row).collect()
    }

    pub async fn retrieve_owner_refund(
        &self,
        query: RefundDetailQuery,
    ) -> Result<Option<RefundView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT r.id, r.refund_no, r.order_id, r.payment_attempt_id,
                   CAST(r.amount AS TEXT) AS amount, r.currency_code, r.status, r.refund_reason_code
            FROM commerce_refund r
            INNER JOIN commerce_order o
                ON o.tenant_id = r.tenant_id
               AND o.id = r.order_id
            WHERE r.tenant_id = CAST(? AS TEXT)
              AND ((r.organization_id = CAST(? AS TEXT)) OR (r.organization_id IS NULL AND ? IS NULL))
              AND o.owner_user_id = CAST(? AS TEXT)
              AND r.id = CAST(? AS TEXT)
            LIMIT 1
            "#,
        )
        .bind(&query.tenant_id)
        .bind(query.organization_id.as_deref())
        .bind(query.organization_id.as_deref())
        .bind(&query.owner_user_id)
        .bind(&query.refund_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to retrieve owner refund", error))?;

        row.map(map_refund_row).transpose()
    }

    async fn find_refund_by_idempotency(
        &self,
        command: &CreateOwnerRefundCommand,
    ) -> Result<Option<RefundView>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id, refund_no, order_id, payment_attempt_id,
                   CAST(amount AS TEXT) AS amount, currency_code, status, refund_reason_code
            FROM commerce_refund
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
        .map_err(|error| store_error("failed to load refund idempotency replay", error))?;

        row.map(map_refund_row).transpose()
    }

    async fn find_latest_succeeded_payment_attempt(
        &self,
        command: &CreateOwnerRefundCommand,
    ) -> Result<Option<String>, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT id
            FROM commerce_payment_attempt
            WHERE tenant_id = CAST(? AS TEXT)
              AND owner_user_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND status = 'succeeded'
            ORDER BY created_at DESC, id DESC
            LIMIT 1
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.order_id)
        .fetch_optional(self.pool())
        .await
        .map_err(|error| store_error("failed to load payment attempt for refund", error))?;

        Ok(row.map(|row| string_cell(&row, "id")))
    }

    /// C4 修复：累计该订单下已发起/处理中/成功的退款金额（minor cents）。
    async fn sum_refunded_amount(
        &self,
        command: &CreateOwnerRefundCommand,
    ) -> Result<i64, CommerceServiceError> {
        let row = sqlx::query(
            r#"
            SELECT COALESCE(SUM(CAST(amount AS REAL)), 0) AS refunded_total
            FROM commerce_refund
            WHERE tenant_id = CAST(? AS TEXT)
              AND order_id = CAST(? AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('submitted', 'processing', 'succeeded')
            "#,
        )
        .bind(&command.tenant_id)
        .bind(&command.order_id)
        .fetch_one(self.pool())
        .await
        .map_err(|error| store_error("failed to sum refunded amount", error))?;

        let total_str = string_cell(&row, "refunded_total");
        money_to_minor_cents(&total_str)
    }
}

async fn insert_refund_event(
    tx: &mut Transaction<'_, Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    refund_id: &str,
    event_type: &str,
    to_status: &str,
    request_no: &str,
    idempotency_key: &str,
    now: &str,
) -> Result<(), CommerceServiceError> {
    let event_id = stable_storage_id(&[
        "refund-event",
        tenant_id,
        refund_id,
        event_type,
        idempotency_key,
    ]);
    let event_no = format!("RFE-{event_type}-{request_no}");
    sqlx::query(
        r#"
        INSERT INTO commerce_refund_event
            (id, tenant_id, organization_id, event_no, refund_id, event_type,
             from_status, to_status, actor_type, actor_id, request_id, idempotency_key, created_at)
        VALUES
            (?, ?, ?, ?, ?, ?, NULL, ?, 'buyer', NULL, ?, ?, ?)
        "#,
    )
    .bind(&event_id)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(&event_no)
    .bind(refund_id)
    .bind(event_type)
    .bind(to_status)
    .bind(request_no)
    .bind(idempotency_key)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert refund event", error))?;
    Ok(())
}

fn map_refund_row(row: sqlx::sqlite::SqliteRow) -> Result<RefundView, CommerceServiceError> {
    Ok(RefundView {
        refund_id: string_cell(&row, "id"),
        refund_no: string_cell(&row, "refund_no"),
        order_id: string_cell(&row, "order_id"),
        payment_attempt_id: string_cell(&row, "payment_attempt_id"),
        amount: CommerceMoney::new(&string_cell(&row, "amount"))
            .map_err(CommerceServiceError::storage)?,
        currency_code: string_cell(&row, "currency_code"),
        status: string_cell(&row, "status"),
        reason_code: optional_string_cell(&row, "refund_reason_code"),
    })
}

fn refund_id(command: &CreateOwnerRefundCommand) -> String {
    stable_storage_id(&[
        "refund",
        &command.tenant_id,
        &command.order_id,
        &command.idempotency_key,
    ])
}

fn optional_string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::sqlite::SqliteRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}
