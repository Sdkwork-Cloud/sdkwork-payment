//! Read-only `commerce_order` snapshots for payment validation.
//!
//! Payment must not depend on `sdkwork-order` crates. These queries are foreign-key
//! lookups only; order lifecycle mutations remain in the order capability.

use sdkwork_contract_service::{CommerceMoney, CommerceServiceError};
use sdkwork_payment_service::OrderPaymentReferenceQuery;
use sdkwork_payment_service::OrderPaymentReferenceSnapshot;
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::{Postgres, Row, Sqlite, Transaction};

use crate::shared::{store_error, string_cell, StringCellRow};

pub(crate) async fn load_order_payment_reference_sqlite(
    tx: &mut Transaction<'_, Sqlite>,
    query: &OrderPaymentReferenceQuery,
) -> Result<Option<OrderPaymentReferenceSnapshot>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT
            o.id AS order_id,
            o.order_no AS order_sn,
            o.subject AS order_subject,
            o.status,
            o.paid_at AS pay_time,
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
    .bind(&query.order_id)
    .bind(&query.tenant_id)
    .bind(query.organization_id.as_deref())
    .bind(query.organization_id.as_deref())
    .bind(&query.owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load order payment reference", error))?;

    Ok(row.map(map_sqlite_order_payment_reference_row))
}

pub(crate) async fn load_order_payment_reference_postgres(
    tx: &mut Transaction<'_, Postgres>,
    query: &OrderPaymentReferenceQuery,
) -> Result<Option<OrderPaymentReferenceSnapshot>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT
            o.id AS order_id,
            o.order_no AS order_sn,
            o.subject AS order_subject,
            o.status,
            o.paid_at AS pay_time,
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
          AND ((o.organization_id = CAST($3 AS TEXT)) OR (o.organization_id IS NULL AND $3::text IS NULL))
          AND o.owner_user_id = CAST($4 AS TEXT)
        "#,
    )
    .bind(&query.order_id)
    .bind(&query.tenant_id)
    .bind(query.organization_id.as_deref())
    .bind(&query.owner_user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to load order payment reference", error))?;

    Ok(row.map(map_postgres_order_payment_reference_row))
}

fn map_sqlite_order_payment_reference_row(row: SqliteRow) -> OrderPaymentReferenceSnapshot {
    map_order_payment_reference_row(&row, optional_sqlite_string_cell(&row, "order_subject"), optional_sqlite_string_cell(&row, "pay_time"))
}

fn map_postgres_order_payment_reference_row(row: PgRow) -> OrderPaymentReferenceSnapshot {
    map_order_payment_reference_row(&row, optional_postgres_string_cell(&row, "order_subject"), optional_postgres_string_cell(&row, "pay_time"))
}

fn map_order_payment_reference_row<R: StringCellRow>(
    row: &R,
    order_subject: Option<String>,
    pay_time: Option<String>,
) -> OrderPaymentReferenceSnapshot {
    OrderPaymentReferenceSnapshot {
        order_id: string_cell(row, "order_id"),
        order_sn: string_cell(row, "order_sn"),
        order_subject,
        status: string_cell(row, "status"),
        total_amount: CommerceMoney::new(&string_cell(row, "total_amount"))
            .unwrap_or_else(|_| CommerceMoney::new("0").expect("zero amount")),
        pay_time,
    }
}

fn optional_sqlite_string_cell(row: &SqliteRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn optional_postgres_string_cell(row: &PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

pub(crate) fn order_status_is_payable(status: &str) -> bool {
    matches!(
        status.trim().to_ascii_lowercase().as_str(),
        "draft"
            | "pending"
            | "pending_payment"
            | "unpaid"
            | "wait_pay"
            | "created"
    )
}

pub(crate) fn order_status_is_refundable(status: &str, pay_time: Option<&str>) -> bool {
    let normalized = status.trim().to_ascii_lowercase();
    let paid = matches!(
        normalized.as_str(),
        "paid" | "succeeded" | "success" | "completed" | "finished"
    );
    paid && pay_time.map(|value| !value.trim().is_empty()).unwrap_or(false)
}
