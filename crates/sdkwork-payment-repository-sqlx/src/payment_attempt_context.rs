use sdkwork_contract_service::CommerceServiceError;
use sqlx::{Pool, Postgres, Row, Sqlite};

use crate::shared::{store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentAttemptProviderContext {
    pub attempt_id: String,
    pub provider_code: String,
    pub out_trade_no: String,
    pub amount: String,
    pub tenant_id: String,
}

pub async fn load_payment_attempt_provider_context_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    owner_user_id: &str,
    payment_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST(? AS TEXT)
          AND owner_user_id = CAST(? AS TEXT)
          AND id = CAST(? AS TEXT)
        "#,
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(payment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt provider context", error))?;

    Ok(row.map(|row| PaymentAttemptProviderContext {
        attempt_id: string_cell(&row, "id"),
        provider_code: string_cell(&row, "provider_code"),
        out_trade_no: string_cell(&row, "out_trade_no"),
        amount: string_cell(&row, "amount"),
        tenant_id: string_cell(&row, "tenant_id"),
    }))
}

pub async fn load_payment_attempt_provider_context_by_id_sqlite(
    pool: &Pool<Sqlite>,
    payment_attempt_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id
        FROM commerce_payment_attempt
        WHERE id = CAST(? AS TEXT)
        "#,
    )
    .bind(payment_attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        store_error(
            "failed to load payment attempt provider context by id",
            error,
        )
    })?;

    Ok(row.map(|row| PaymentAttemptProviderContext {
        attempt_id: string_cell(&row, "id"),
        provider_code: string_cell(&row, "provider_code"),
        out_trade_no: string_cell(&row, "out_trade_no"),
        amount: string_cell(&row, "amount"),
        tenant_id: string_cell(&row, "tenant_id"),
    }))
}

pub async fn load_payment_attempt_provider_context_postgres(
    pool: &Pool<Postgres>,
    tenant_id: &str,
    owner_user_id: &str,
    payment_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id
        FROM commerce_payment_attempt
        WHERE tenant_id = CAST($1 AS TEXT)
          AND owner_user_id = CAST($2 AS TEXT)
          AND id = CAST($3 AS TEXT)
        "#,
    )
    .bind(tenant_id)
    .bind(owner_user_id)
    .bind(payment_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment attempt provider context", error))?;

    Ok(row.map(|row| PaymentAttemptProviderContext {
        attempt_id: string_cell(&row, "id"),
        provider_code: string_cell(&row, "provider_code"),
        out_trade_no: string_cell(&row, "out_trade_no"),
        amount: string_cell(&row, "amount"),
        tenant_id: string_cell(&row, "tenant_id"),
    }))
}

pub async fn load_payment_attempt_provider_context_by_id_postgres(
    pool: &Pool<Postgres>,
    payment_attempt_id: &str,
) -> Result<Option<PaymentAttemptProviderContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, provider_code, out_trade_no, amount, tenant_id
        FROM commerce_payment_attempt
        WHERE id = CAST($1 AS TEXT)
        "#,
    )
    .bind(payment_attempt_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| {
        store_error(
            "failed to load payment attempt provider context by id",
            error,
        )
    })?;

    Ok(row.map(|row| PaymentAttemptProviderContext {
        attempt_id: string_cell(&row, "id"),
        provider_code: string_cell(&row, "provider_code"),
        out_trade_no: string_cell(&row, "out_trade_no"),
        amount: string_cell(&row, "amount"),
        tenant_id: string_cell(&row, "tenant_id"),
    }))
}

/// Payment-attempt context returned after webhook ingest (no order-table join).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentWebhookAttemptContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub owner_user_id: String,
    pub order_id: String,
}

pub(crate) async fn load_payment_webhook_attempt_context_by_out_trade_no_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    out_trade_no: &str,
) -> Result<Option<PaymentWebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, owner_user_id, order_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve payment webhook attempt context", error))?;

    Ok(row.map(|row| PaymentWebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        owner_user_id: string_cell(&row, "owner_user_id"),
        order_id: string_cell(&row, "order_id"),
    }))
}

pub(crate) async fn load_payment_webhook_attempt_context_by_out_trade_no_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    out_trade_no: &str,
) -> Result<Option<PaymentWebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, owner_user_id, order_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve payment webhook attempt context", error))?;

    Ok(row.map(|row| PaymentWebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        owner_user_id: string_cell(&row, "owner_user_id"),
        order_id: string_cell(&row, "order_id"),
    }))
}

pub(crate) async fn load_attempt_by_out_trade_no_sqlite(
    tx: &mut sqlx::Transaction<'_, Sqlite>,
    out_trade_no: &str,
) -> Result<Option<(String, String)>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve webhook tenant", error))?;

    Ok(row.map(|row| (string_cell(&row, "tenant_id"), string_cell(&row, "id"))))
}

pub(crate) async fn load_attempt_by_out_trade_no_postgres(
    tx: &mut sqlx::Transaction<'_, Postgres>,
    out_trade_no: &str,
) -> Result<Option<(String, String)>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT id, tenant_id
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to resolve webhook tenant", error))?;

    Ok(row.map(|row| (string_cell(&row, "tenant_id"), string_cell(&row, "id"))))
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebhookAttemptContext {
    pub tenant_id: String,
    pub organization_id: Option<String>,
    pub provider_code: String,
}

pub async fn load_webhook_attempt_context_by_out_trade_no_sqlite(
    pool: &Pool<Sqlite>,
    out_trade_no: &str,
) -> Result<Option<WebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST(? AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load webhook attempt context", error))?;

    Ok(row.map(|row| WebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    }))
}

pub async fn load_webhook_attempt_context_by_out_trade_no_postgres(
    pool: &Pool<Postgres>,
    out_trade_no: &str,
) -> Result<Option<WebhookAttemptContext>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code
        FROM commerce_payment_attempt
        WHERE out_trade_no = CAST($1 AS TEXT)
        ORDER BY created_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(out_trade_no)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load webhook attempt context", error))?;

    Ok(row.map(|row| WebhookAttemptContext {
        tenant_id: string_cell(&row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    }))
}
