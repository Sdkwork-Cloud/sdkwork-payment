#![allow(clippy::too_many_arguments)]

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_payment_service::{
    CreateOwnerRefundCommand, OrderPaymentReferenceQuery, RefundDetailQuery, RefundListPage,
    RefundListQuery, RefundView,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};

use crate::order_reference::{load_order_payment_reference_sqlite, order_status_is_refundable};
use crate::shared::{
    current_timestamp_string, ensure_refund_status_transition, money_to_minor_units,
    resolve_refund_amount, stable_storage_id, store_error, validate_refund_bounds,
};

#[derive(Debug, Clone)]
pub struct SqliteCommerceRefundStore {
    pool: SqlitePool,
}

impl SqliteCommerceRefundStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
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

        let mut tx = self
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| store_error("failed to begin refund transaction", error))?;

        let reference_query = OrderPaymentReferenceQuery::new(
            &command.tenant_id,
            command.organization_id.as_deref(),
            &command.owner_user_id,
            &command.order_id,
        )?;
        let Some(order_ref) =
            load_order_payment_reference_sqlite(&mut tx, &reference_query).await?
        else {
            return Err(CommerceServiceError::not_found("order was not found"));
        };
        if !order_status_is_refundable(&order_ref.status, order_ref.pay_time.as_deref()) {
            return Err(CommerceServiceError::conflict(
                "order is not eligible for refund",
            ));
        }

        let payment_attempt_id = match command.payment_attempt_id.as_deref() {
            Some(value) => value.to_owned(),
            None => find_latest_succeeded_payment_attempt_in_tx(&mut tx, &command)
                .await?
                .ok_or_else(|| CommerceServiceError::not_found("payment attempt was not found"))?,
        };

        let refund_amount = resolve_refund_amount(&command, &order_ref.total_amount)?;
        let total_minor = money_to_minor_units(order_ref.total_amount.as_str())?;
        let refund_minor = money_to_minor_units(&refund_amount)?;
        validate_refund_bounds(refund_minor, total_minor)?;
        let already_refunded_minor = sum_refunded_amount_in_tx(&mut tx, &command).await?;
        if refund_minor > total_minor.saturating_sub(already_refunded_minor) {
            return Err(CommerceServiceError::conflict(
                "refund amount exceeds remaining refundable amount",
            ));
        }

        let now = current_timestamp_string();
        let refund_id = refund_id(&command);
        let refund_no = format!("RF-{}", command.request_no);
        ensure_refund_status_transition(None, "submitted")?;

        let insert_result = sqlx::query(
            r#"
            INSERT INTO commerce_refund
                (id, tenant_id, organization_id, order_id, payment_attempt_id, refund_no,
                 amount, currency_code, status, refund_reason_code, requested_by_type,
                 requested_by, request_no, idempotency_key, created_at, updated_at)
            VALUES
                (?, ?, ?, ?, ?, ?, ?, ?, 'submitted', ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT (id) DO NOTHING
            "#,
        )
        .bind(&refund_id)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.order_id)
        .bind(&payment_attempt_id)
        .bind(&refund_no)
        .bind(&refund_amount)
        .bind(&command.currency_code)
        .bind(command.reason_code.as_deref())
        .bind(&command.requested_by_type)
        .bind(&command.requested_by)
        .bind(&command.request_no)
        .bind(&command.idempotency_key)
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to insert refund", error))?;

        if insert_result.rows_affected() == 0 {
            if let Some(existing) = find_refund_by_idempotency_in_tx(&mut tx, &command).await? {
                tx.commit().await.map_err(|error| {
                    store_error("failed to commit refund idempotency replay", error)
                })?;
                return Ok(existing);
            }
            return Err(CommerceServiceError::conflict(
                "refund idempotency identity could not be resolved",
            ));
        }

        insert_refund_event(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
            &refund_id,
            "created",
            None,
            "submitted",
            &command.requested_by_type,
            Some(&command.requested_by),
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
            currency_code: command.currency_code,
            status: "submitted".to_owned(),
            reason_code: command.reason_code,
        })
    }

    pub async fn list_owner_refunds(
        &self,
        query: RefundListQuery,
    ) -> Result<RefundListPage, CommerceServiceError> {
        let mut sql = String::from(
            r#"
            SELECT r.id, r.refund_no, r.order_id, r.payment_attempt_id,
                   CAST(r.amount AS TEXT) AS amount, r.currency_code, r.status, r.refund_reason_code,
                   COUNT(*) OVER() AS total_count
            FROM commerce_refund r
            INNER JOIN commerce_order o
                ON o.tenant_id = r.tenant_id
               AND o.id = r.order_id
            WHERE r.tenant_id = CAST(? AS TEXT)
              AND ((r.organization_id = CAST(? AS TEXT)) OR (r.organization_id IS NULL AND ? IS NULL))
              AND o.owner_user_id = CAST(? AS TEXT)
              AND r.deleted_at IS NULL
            "#,
        );
        if query.status.is_some() {
            sql.push_str(" AND r.status = CAST(? AS TEXT)");
        }
        sql.push_str(" ORDER BY r.created_at DESC, r.id DESC LIMIT ? OFFSET ?");

        let mut db_query = sqlx::query(&sql)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id);
        if let Some(status) = query.status.as_deref() {
            db_query = db_query.bind(status);
        }
        db_query = db_query.bind(query.limit).bind(query.offset);

        let rows = db_query
            .fetch_all(self.pool())
            .await
            .map_err(|error| store_error("failed to list owner refunds", error))?;

        let total_items = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows
            .into_iter()
            .map(map_refund_row)
            .collect::<Result<Vec<_>, _>>()?;

        Ok(RefundListPage { items, total_items })
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

    pub async fn mark_owner_refund_provider_submission_failed(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        refund_id: &str,
        actor_type: &str,
        actor_id: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<RefundView, CommerceServiceError> {
        let now = current_timestamp_string();
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| {
                store_error("failed to begin refund provider failure transaction", error)
            })?;

        let row = sqlx::query(
            r#"
            SELECT id, refund_no, order_id, payment_attempt_id,
                   CAST(amount AS TEXT) AS amount, currency_code, status, refund_reason_code
            FROM commerce_refund
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(refund_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load refund for provider failure", error))?;

        let Some(row) = row else {
            return Err(CommerceServiceError::not_found("refund was not found"));
        };

        let current_status = string_cell(&row, "status");
        ensure_refund_status_transition(Some(&current_status), "failed")?;

        sqlx::query(
            r#"
            UPDATE commerce_refund
            SET status = 'failed', updated_at = ?, version = version + 1
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND id = CAST(? AS TEXT)
              AND status IN ('submitted', 'processing')
            "#,
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(refund_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| store_error("failed to mark refund provider submission failed", error))?;

        insert_refund_event(
            &mut tx,
            tenant_id,
            organization_id,
            refund_id,
            "failed",
            Some(&current_status),
            "failed",
            actor_type,
            actor_id,
            request_no,
            idempotency_key,
            &now,
        )
        .await?;

        tx.commit().await.map_err(|error| {
            store_error(
                "failed to commit refund provider failure transaction",
                error,
            )
        })?;

        map_refund_row(row).map(|mut view| {
            view.status = "failed".to_owned();
            view
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn mark_owner_refund_provider_submission_processing(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
        refund_id: &str,
        actor_type: &str,
        actor_id: Option<&str>,
        request_no: &str,
        idempotency_key: &str,
    ) -> Result<RefundView, CommerceServiceError> {
        let now = current_timestamp_string();
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|error| store_error("failed to begin refund submission transaction", error))?;
        let row = sqlx::query(
            r#"
            SELECT id, refund_no, order_id, payment_attempt_id,
                   CAST(amount AS TEXT) AS amount, currency_code, status, refund_reason_code
            FROM commerce_refund
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND id = CAST(? AS TEXT)
              AND deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(refund_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|error| store_error("failed to load refund for provider submission", error))?;
        let Some(row) = row else {
            return Err(CommerceServiceError::not_found("refund was not found"));
        };
        let current_status = string_cell(&row, "status");
        ensure_refund_status_transition(Some(&current_status), "processing")?;
        let result = sqlx::query(
            r#"
            UPDATE commerce_refund
            SET status = 'processing', updated_at = ?, version = version + 1
            WHERE tenant_id = CAST(? AS TEXT)
              AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
              AND id = CAST(? AS TEXT)
              AND status IN ('submitted', 'failed')
            "#,
        )
        .bind(&now)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(organization_id)
        .bind(refund_id)
        .execute(&mut *tx)
        .await
        .map_err(|error| {
            store_error(
                "failed to mark refund provider submission processing",
                error,
            )
        })?;
        if result.rows_affected() != 1 {
            return Err(CommerceServiceError::conflict(
                "refund is already processing or is not retryable",
            ));
        }
        insert_refund_event(
            &mut tx,
            tenant_id,
            organization_id,
            refund_id,
            "status_changed",
            Some(&current_status),
            "processing",
            actor_type,
            actor_id,
            request_no,
            idempotency_key,
            &now,
        )
        .await?;
        tx.commit().await.map_err(|error| {
            store_error("failed to commit refund submission transaction", error)
        })?;
        map_refund_row(row).map(|mut view| {
            view.status = "processing".to_owned();
            view
        })
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
}

async fn find_latest_succeeded_payment_attempt_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    command: &CreateOwnerRefundCommand,
) -> Result<Option<String>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST(? AS TEXT)
          AND owner_user_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('succeeded', 'success', 'paid')
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.owner_user_id)
    .bind(&command.order_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load payment attempt for refund", error))?;

    Ok(row.map(|row| string_cell(&row, "id")))
}

async fn sum_refunded_amount_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
    command: &CreateOwnerRefundCommand,
) -> Result<i64, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT CAST(amount AS TEXT) AS amount
        FROM commerce_refund
        WHERE tenant_id = CAST(? AS TEXT)
          AND order_id = CAST(? AS TEXT)
          AND LOWER(COALESCE(status, '')) IN ('submitted', 'processing', 'succeeded')
          AND deleted_at IS NULL
        "#,
    )
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .fetch_all(&mut **tx)
    .await
    .map_err(|error| store_error("failed to sum refunded amount", error))?;

    rows.iter().try_fold(0_i64, |acc, row| {
        let amount = string_cell(row, "amount");
        let minor = money_to_minor_units(&amount)?;
        acc.checked_add(minor)
            .ok_or_else(|| CommerceServiceError::validation("refunded amount sum overflows i64"))
    })
}

async fn find_refund_by_idempotency_in_tx(
    tx: &mut Transaction<'_, Sqlite>,
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
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load refund idempotency replay in tx", error))?;

    row.map(map_refund_row).transpose()
}

async fn insert_refund_event(
    tx: &mut Transaction<'_, Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    refund_id: &str,
    event_type: &str,
    from_status: Option<&str>,
    to_status: &str,
    actor_type: &str,
    actor_id: Option<&str>,
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
            (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&event_id)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(&event_no)
    .bind(refund_id)
    .bind(event_type)
    .bind(from_status)
    .bind(to_status)
    .bind(actor_type)
    .bind(actor_id)
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
