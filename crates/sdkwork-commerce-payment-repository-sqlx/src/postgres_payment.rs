use sdkwork_commerce_contract_service::{
    CommerceMoney, CommercePaymentStatus, CommerceRechargeStatus, CommerceServiceError,
};
use sdkwork_commerce_payment_service::{
    ClosePaymentRecordCommand, PaymentRecordDetailQuery, PaymentRecordItem, PaymentRecordListQuery,
    PaymentRecordOrderListQuery,
};
use sqlx::{PgPool, Row};

const LIST_PAYMENT_RECORDS: &str = r#"
SELECT
    CAST(o.id AS TEXT) AS order_id,
    CAST(COALESCE(pa.id, pi.id, o.id) AS TEXT) AS id,
    CAST(pi.id AS TEXT) AS payment_id,
    CAST(pa.id AS TEXT) AS payment_attempt_id,
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, ''), '-') AS order_no,
    COALESCE(NULLIF(pa.payment_method, ''), NULLIF(pi.payment_method, ''), '-') AS method,
    CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) AS amount,
    CAST(COALESCE(pa.paid_at, pa.created_at, o.paid_at, o.created_at) AS TEXT) AS date,
    o.status AS order_status,
    pi.status AS payment_status,
    pa.status AS payment_attempt_status
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.owner_user_id = o.owner_user_id
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.owner_user_id = o.owner_user_id
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
ORDER BY COALESCE(pa.paid_at, pa.created_at, o.paid_at, o.created_at) DESC NULLS LAST, o.id DESC
LIMIT 100
"#;

const LIST_PAYMENT_RECORDS_BY_ORDER: &str = r#"
SELECT
    CAST(o.id AS TEXT) AS order_id,
    CAST(COALESCE(pa.id, pi.id, o.id) AS TEXT) AS id,
    CAST(pi.id AS TEXT) AS payment_id,
    CAST(pa.id AS TEXT) AS payment_attempt_id,
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, ''), '-') AS order_no,
    COALESCE(NULLIF(pa.payment_method, ''), NULLIF(pi.payment_method, ''), '-') AS method,
    CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) AS amount,
    CAST(COALESCE(pa.paid_at, pa.created_at, o.paid_at, o.created_at) AS TEXT) AS date,
    o.status AS order_status,
    pi.status AS payment_status,
    pa.status AS payment_attempt_status
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.owner_user_id = o.owner_user_id
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.owner_user_id = o.owner_user_id
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.id = CAST($4 AS TEXT)
ORDER BY COALESCE(pa.paid_at, pa.created_at, o.paid_at, o.created_at) DESC NULLS LAST, o.id DESC
LIMIT 100
"#;

const RETRIEVE_PAYMENT_RECORD: &str = r#"
SELECT
    CAST(o.id AS TEXT) AS order_id,
    CAST(COALESCE(pa.id, pi.id, o.id) AS TEXT) AS id,
    CAST(pi.id AS TEXT) AS payment_id,
    CAST(pa.id AS TEXT) AS payment_attempt_id,
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, ''), '-') AS order_no,
    COALESCE(NULLIF(pa.payment_method, ''), NULLIF(pi.payment_method, ''), '-') AS method,
    CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) AS amount,
    CAST(COALESCE(pa.paid_at, pa.created_at, o.paid_at, o.created_at) AS TEXT) AS date,
    o.status AS order_status,
    pi.status AS payment_status,
    pa.status AS payment_attempt_status
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.owner_user_id = o.owner_user_id
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.owner_user_id = o.owner_user_id
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND (pa.id = CAST($4 AS TEXT) OR pi.id = CAST($4 AS TEXT) OR o.id = CAST($4 AS TEXT))
LIMIT 1
"#;

#[derive(Debug, Clone)]
pub struct PostgresCommercePaymentRecordStore {
    pool: PgPool,
}

impl PostgresCommercePaymentRecordStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_payment_records(
        &self,
        query: PaymentRecordListQuery,
    ) -> Result<Vec<PaymentRecordItem>, CommerceServiceError> {
        let rows = sqlx::query(LIST_PAYMENT_RECORDS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .fetch_all(&self.pool)
            .await
            .or_else(empty_rows_when_read_model_is_missing)
            .map_err(|error| store_error("failed to list payment records", error))?;

        rows.iter().map(payment_record_from_row).collect()
    }

    pub async fn list_payment_records_by_order(
        &self,
        query: PaymentRecordOrderListQuery,
    ) -> Result<Vec<PaymentRecordItem>, CommerceServiceError> {
        let rows = sqlx::query(LIST_PAYMENT_RECORDS_BY_ORDER)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.order_id)
            .fetch_all(&self.pool)
            .await
            .or_else(empty_rows_when_read_model_is_missing)
            .map_err(|error| store_error("failed to list payment records by order", error))?;

        rows.iter().map(payment_record_from_row).collect()
    }

    pub async fn retrieve_payment_record(
        &self,
        query: PaymentRecordDetailQuery,
    ) -> Result<PaymentRecordItem, CommerceServiceError> {
        let row = sqlx::query(RETRIEVE_PAYMENT_RECORD)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.payment_id)
            .fetch_optional(&self.pool)
            .await
            .or_else(none_when_read_model_is_missing)
            .map_err(|error| store_error("failed to retrieve payment record", error))?;

        row.as_ref()
            .map(payment_record_from_row)
            .transpose()?
            .ok_or_else(|| CommerceServiceError::not_found("payment record was not found"))
    }
    pub async fn close_payment_record(
        &self,
        command: ClosePaymentRecordCommand,
    ) -> Result<(), CommerceServiceError> {
        let now = format!(
            "{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0)
        );
        let attempt = sqlx::query(
            r#"
            UPDATE commerce_payment_attempt
            SET status = $1, updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.payment_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to close payment attempt", error))?;

        if attempt.rows_affected() > 0 {
            return Ok(());
        }

        let intent = sqlx::query(
            r#"
            UPDATE commerce_payment_intent
            SET status = $1, updated_at = $2
            WHERE tenant_id = CAST($3 AS TEXT)
              AND owner_user_id = CAST($4 AS TEXT)
              AND id = CAST($5 AS TEXT)
              AND LOWER(COALESCE(status, '')) IN ('created', 'pending', 'processing')
            "#,
        )
        .bind(CommercePaymentStatus::Canceled.as_str())
        .bind(&now)
        .bind(&command.tenant_id)
        .bind(&command.owner_user_id)
        .bind(&command.payment_id)
        .execute(&self.pool)
        .await
        .map_err(|error| store_error("failed to close payment intent", error))?;

        if intent.rows_affected() == 0 {
            return Err(CommerceServiceError::conflict(
                "payment record is not closable or was not found",
            ));
        }

        Ok(())
    }
}

fn payment_record_from_row(
    row: &sqlx::postgres::PgRow,
) -> Result<PaymentRecordItem, CommerceServiceError> {
    PaymentRecordItem::new(
        &string_cell(row, "id"),
        &string_cell(row, "order_id"),
        &string_cell(row, "order_no"),
        &string_cell(row, "method"),
        commerce_money_cell(row, "amount", "payment record amount")?,
        &string_cell(row, "date"),
        payment_record_status(row)?,
    )
}

fn payment_record_status(
    row: &sqlx::postgres::PgRow,
) -> Result<&'static str, CommerceServiceError> {
    let order_status =
        order_recharge_status_label(&required_status_cell(row, "order_status", "order")?)?;
    let payment_status = related_status_cell(row, "payment_id", "payment_status", "payment")?
        .map(|status| payment_status_label(&status))
        .transpose()?
        .unwrap_or("pending");
    let payment_attempt_status = related_status_cell(
        row,
        "payment_attempt_id",
        "payment_attempt_status",
        "payment attempt",
    )?
    .map(|status| payment_status_label(&status))
    .transpose()?
    .unwrap_or("pending");

    Ok(payment_record_status_label(
        order_status,
        payment_status,
        payment_attempt_status,
    ))
}

fn payment_record_status_label(
    order_status: &str,
    payment_status: &str,
    payment_attempt_status: &str,
) -> &'static str {
    if order_status == "failed" {
        "failed"
    } else if payment_attempt_status == "success"
        || payment_status == "success"
        || order_status == "success"
    {
        "success"
    } else if payment_attempt_status == "failed" || payment_status == "failed" {
        "failed"
    } else {
        "pending"
    }
}

fn order_recharge_status_label(value: &str) -> Result<&'static str, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        status if status == CommerceRechargeStatus::Pending.as_str() => Ok("pending"),
        status if status == CommerceRechargeStatus::Paid.as_str() => Ok("success"),
        status if status == CommerceRechargeStatus::Fulfilled.as_str() => Ok("success"),
        status if status == CommerceRechargeStatus::Closed.as_str() => Ok("failed"),
        "pending_payment" | "unpaid" | "wait_pay" | "draft" => Ok("pending"),
        "paid" | "success" | "completed" => Ok("success"),
        "cancelled" | "canceled" | "closed" | "failed" => Ok("failed"),
        status => Err(CommerceServiceError::storage(format!(
            "unsupported payment record order status: {status}"
        ))),
    }
}

fn payment_status_label(value: &str) -> Result<&'static str, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        status if status == CommercePaymentStatus::Pending.as_str() => Ok("pending"),
        status if status == CommercePaymentStatus::Succeeded.as_str() => Ok("success"),
        status if status == CommercePaymentStatus::Failed.as_str() => Ok("failed"),
        status if status == CommercePaymentStatus::Canceled.as_str() => Ok("failed"),
        status => Err(CommerceServiceError::storage(format!(
            "unsupported payment record payment status: {status}"
        ))),
    }
}

fn optional_string_cell(row: &sqlx::postgres::PgRow, column: &str) -> Option<String> {
    row.try_get::<Option<String>, _>(column).ok().flatten()
}

fn string_cell(row: &sqlx::postgres::PgRow, column: &str) -> String {
    optional_string_cell(row, column).unwrap_or_default()
}

fn required_status_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    source: &str,
) -> Result<String, CommerceServiceError> {
    optional_string_cell(row, column)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| missing_payment_record_status_error(source))
}

fn related_status_cell(
    row: &sqlx::postgres::PgRow,
    relation_column: &str,
    status_column: &str,
    source: &str,
) -> Result<Option<String>, CommerceServiceError> {
    if optional_string_cell(row, relation_column)
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        return Ok(None);
    }
    required_status_cell(row, status_column, source).map(Some)
}

fn missing_payment_record_status_error(source: &str) -> CommerceServiceError {
    CommerceServiceError::storage(format!(
        "missing payment record {source} status from database row"
    ))
}

fn commerce_money_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
    field_name: &str,
) -> Result<CommerceMoney, CommerceServiceError> {
    let value = string_cell(row, column);
    let cents = money_cents(&value)
        .map_err(|_| CommerceServiceError::storage(format!("invalid {field_name}: {value}")))?;
    CommerceMoney::new(&format_money_minor(cents))
        .map_err(|message| CommerceServiceError::storage(format!("{message}: {value}")))
}

fn money_cents(amount: &str) -> Result<i64, CommerceServiceError> {
    let value = amount.trim();
    let mut parts = value.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<i64>()
        .map_err(|_| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > 2 {
        return Err(CommerceServiceError::storage(format!(
            "invalid commerce money amount: {value}"
        )));
    }
    let mut padded = fraction.to_string();
    while padded.len() < 2 {
        padded.push('0');
    }
    let cents = if padded.is_empty() {
        0
    } else {
        padded.parse::<i64>().map_err(|_| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })?
    };
    whole
        .checked_mul(100)
        .and_then(|amount| amount.checked_add(cents))
        .ok_or_else(|| {
            CommerceServiceError::storage(format!("invalid commerce money amount: {value}"))
        })
}

fn format_money_minor(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
}

fn empty_rows_when_read_model_is_missing(
    error: sqlx::Error,
) -> Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    if is_missing_postgres_read_model(&error) {
        Ok(Vec::new())
    } else {
        Err(error)
    }
}

fn none_when_read_model_is_missing(
    error: sqlx::Error,
) -> Result<Option<sqlx::postgres::PgRow>, sqlx::Error> {
    if is_missing_postgres_read_model(&error) {
        Ok(None)
    } else {
        Err(error)
    }
}

fn is_missing_postgres_read_model(error: &sqlx::Error) -> bool {
    if matches!(error, sqlx::Error::ColumnNotFound(_)) {
        return true;
    }
    error
        .as_database_error()
        .and_then(|database_error| database_error.code())
        .map(|code| matches!(code.as_ref(), "42P01" | "42703"))
        .unwrap_or(false)
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{context}: {error}"))
}
