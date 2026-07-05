use sdkwork_contract_service::CommerceServiceError;
use serde_json::Value;
use sqlx::{Pool, Postgres, Row, Sqlite};

use crate::shared::{store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentProviderAccountRecord {
    pub provider_code: String,
    pub merchant_id: Option<String>,
    pub environment: String,
    pub secret_ref: String,
    pub webhook_secret_ref: Option<String>,
    pub certificate_ref: Option<String>,
    pub metadata: Value,
}

pub async fn load_active_provider_account_sqlite(
    pool: &Pool<Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    provider_code: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT provider_code, merchant_id, environment, secret_ref, webhook_secret_ref,
               certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE tenant_id = CAST(? AS TEXT)
          AND ((organization_id = CAST(? AS TEXT)) OR (organization_id IS NULL AND ? IS NULL))
          AND LOWER(provider_code) = LOWER(CAST(? AS TEXT))
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(organization_id)
    .bind(provider_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account", error))?;

    Ok(row.map(|row| map_provider_account_row_sqlite(&row)))
}

fn metadata_from_row(raw: Result<String, sqlx::Error>) -> Value {
    match raw {
        Ok(text) => serde_json::from_str(&text).unwrap_or(Value::Object(Default::default())),
        Err(_) => Value::Object(Default::default()),
    }
}

fn metadata_from_jsonb(raw: Result<Value, sqlx::Error>) -> Value {
    raw.unwrap_or(Value::Object(Default::default()))
}
fn map_provider_account_row_sqlite(row: &sqlx::sqlite::SqliteRow) -> PaymentProviderAccountRecord {
    let metadata = metadata_from_row(row.try_get("metadata"));
    PaymentProviderAccountRecord {
        provider_code: string_cell(row, "provider_code"),
        merchant_id: row.try_get("merchant_id").ok().flatten(),
        environment: string_cell(row, "environment"),
        secret_ref: string_cell(row, "secret_ref"),
        webhook_secret_ref: row.try_get("webhook_secret_ref").ok().flatten(),
        certificate_ref: row.try_get("certificate_ref").ok().flatten(),
        metadata,
    }
}

pub async fn load_active_provider_account_postgres(
    pool: &Pool<Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    provider_code: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT provider_code, merchant_id, environment, secret_ref, webhook_secret_ref,
               certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE tenant_id = CAST($1 AS TEXT)
          AND ((organization_id = CAST($2 AS TEXT)) OR (organization_id IS NULL AND $2 IS NULL))
          AND LOWER(provider_code) = LOWER(CAST($3 AS TEXT))
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(tenant_id)
    .bind(organization_id)
    .bind(provider_code)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account", error))?;

    Ok(row.map(|row| map_provider_account_row_postgres(&row)))
}

pub async fn load_active_provider_account_by_merchant_id_sqlite(
    pool: &Pool<Sqlite>,
    provider_code: &str,
    merchant_id: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT provider_code, merchant_id, environment, secret_ref, webhook_secret_ref,
               certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE LOWER(provider_code) = LOWER(CAST(? AS TEXT))
          AND merchant_id = CAST(? AS TEXT)
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(provider_code)
    .bind(merchant_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account by merchant", error))?;

    Ok(row.map(|row| map_provider_account_row_sqlite(&row)))
}

pub async fn load_active_provider_account_by_merchant_id_postgres(
    pool: &Pool<Postgres>,
    provider_code: &str,
    merchant_id: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        SELECT provider_code, merchant_id, environment, secret_ref, webhook_secret_ref,
               certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE LOWER(provider_code) = LOWER(CAST($1 AS TEXT))
          AND merchant_id = CAST($2 AS TEXT)
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 1
        "#,
    )
    .bind(provider_code)
    .bind(merchant_id)
    .fetch_optional(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account by merchant", error))?;

    Ok(row.map(|row| map_provider_account_row_postgres(&row)))
}

fn map_provider_account_row_postgres(row: &sqlx::postgres::PgRow) -> PaymentProviderAccountRecord {
    let metadata = metadata_from_jsonb(row.try_get("metadata"));
    PaymentProviderAccountRecord {
        provider_code: string_cell(row, "provider_code"),
        merchant_id: row.try_get("merchant_id").ok().flatten(),
        environment: string_cell(row, "environment"),
        secret_ref: string_cell(row, "secret_ref"),
        webhook_secret_ref: row.try_get("webhook_secret_ref").ok().flatten(),
        certificate_ref: row.try_get("certificate_ref").ok().flatten(),
        metadata,
    }
}
