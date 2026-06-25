#![allow(clippy::too_many_arguments)]

use sdkwork_commerce_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_commerce_order_service::OrderOwnerDetailQuery;
use sdkwork_commerce_order_repository_sqlx::PostgresCommerceOrderStore;
use std::sync::Arc;
use sdkwork_commerce_payment_service::{
    CreateOwnerRefundCommand, RefundDetailQuery, RefundListQuery, RefundView,
};
use sqlx::{PgPool, Postgres, Row, Transaction};



#[derive(Debug, Clone)]
pub struct PostgresCommerceRefundStore {
    pool: PgPool,
    order_store: Arc<PostgresCommerceOrderStore>,
}

impl PostgresCommerceRefundStore {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: pool.clone(),
            order_store: Arc::new(PostgresCommerceOrderStore::new(pool)),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl PostgresCommerceRefundStore {
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
        if detail.summary.status != "paid" && detail.summary.pay_time.is_none() {
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

        let refund_amount = command
            .amount
            .clone()
            .unwrap_or_else(|| detail.summary.total_amount.as_str().to_owned());

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
                ($1, $2, $3, $4, $5, $6, $7, 'CNY', 'submitted', $8, 'buyer', $9, $10, $11, $12, $13)
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
            WHERE r.tenant_id = CAST($1 AS TEXT)
              AND ((r.organization_id = CAST($2 AS TEXT)) OR (r.organization_id IS NULL AND $3 IS NULL))
              AND o.owner_user_id = CAST($4 AS TEXT)
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
            WHERE r.tenant_id = CAST($1 AS TEXT)
              AND ((r.organization_id = CAST($2 AS TEXT)) OR (r.organization_id IS NULL AND $3 IS NULL))
              AND o.owner_user_id = CAST($4 AS TEXT)
              AND r.id = CAST($5 AS TEXT)
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
            WHERE tenant_id = CAST($1 AS TEXT)
              AND order_id = CAST($2 AS TEXT)
              AND idempotency_key = CAST($3 AS TEXT)
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
            WHERE tenant_id = CAST($1 AS TEXT)
              AND owner_user_id = CAST($2 AS TEXT)
              AND order_id = CAST($3 AS TEXT)
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
}

async fn insert_refund_event(
    tx: &mut Transaction<'_, Postgres>,
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
            ($1, $2, $3, $4, $5, $6, NULL, $7, 'buyer', NULL, $8, $9, $10)
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

fn map_refund_row(row: sqlx::postgres::PgRow) -> Result<RefundView, CommerceServiceError> {
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

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}
