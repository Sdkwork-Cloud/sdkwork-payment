use sdkwork_contract_service::CommerceServiceError;
use serde_json::Value;
use sqlx::{Pool, Postgres, Row, Sqlite};

use crate::shared::{store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaymentProviderAccountRecord {
    pub tenant_id: String,
    pub organization_id: Option<String>,
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
        SELECT tenant_id, organization_id, provider_code, merchant_id, environment, secret_ref,
               webhook_secret_ref, certificate_ref, metadata
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
        tenant_id: string_cell(row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
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
        SELECT tenant_id, organization_id, provider_code, merchant_id, environment, secret_ref,
               webhook_secret_ref, certificate_ref, metadata
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
    let rows = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code, merchant_id, environment, secret_ref,
               webhook_secret_ref, certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE LOWER(provider_code) = LOWER(CAST(? AS TEXT))
          AND merchant_id = CAST(? AS TEXT)
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 2
        "#,
    )
    .bind(provider_code)
    .bind(merchant_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account by merchant", error))?;

    match rows.as_slice() {
        [] => Ok(None),
        [row] => Ok(Some(map_provider_account_row_sqlite(row))),
        _ => Err(CommerceServiceError::conflict(
            "multiple active payment provider accounts match merchant identity",
        )),
    }
}

pub async fn load_active_provider_account_by_merchant_id_postgres(
    pool: &Pool<Postgres>,
    provider_code: &str,
    merchant_id: &str,
) -> Result<Option<PaymentProviderAccountRecord>, CommerceServiceError> {
    let rows = sqlx::query(
        r#"
        SELECT tenant_id, organization_id, provider_code, merchant_id, environment, secret_ref,
               webhook_secret_ref, certificate_ref, metadata
        FROM commerce_payment_provider_account
        WHERE LOWER(provider_code) = LOWER(CAST($1 AS TEXT))
          AND merchant_id = CAST($2 AS TEXT)
          AND status = 'active'
          AND deleted_at IS NULL
        ORDER BY updated_at DESC, id DESC
        LIMIT 2
        "#,
    )
    .bind(provider_code)
    .bind(merchant_id)
    .fetch_all(pool)
    .await
    .map_err(|error| store_error("failed to load payment provider account by merchant", error))?;

    match rows.as_slice() {
        [] => Ok(None),
        [row] => Ok(Some(map_provider_account_row_postgres(row))),
        _ => Err(CommerceServiceError::conflict(
            "multiple active payment provider accounts match merchant identity",
        )),
    }
}

fn map_provider_account_row_postgres(row: &sqlx::postgres::PgRow) -> PaymentProviderAccountRecord {
    let metadata = metadata_from_jsonb(row.try_get("metadata"));
    PaymentProviderAccountRecord {
        tenant_id: string_cell(row, "tenant_id"),
        organization_id: row.try_get("organization_id").ok().flatten(),
        provider_code: string_cell(row, "provider_code"),
        merchant_id: row.try_get("merchant_id").ok().flatten(),
        environment: string_cell(row, "environment"),
        secret_ref: string_cell(row, "secret_ref"),
        webhook_secret_ref: row.try_get("webhook_secret_ref").ok().flatten(),
        certificate_ref: row.try_get("certificate_ref").ok().flatten(),
        metadata,
    }
}

#[cfg(test)]
mod tests {
    use super::load_active_provider_account_by_merchant_id_sqlite;

    #[tokio::test]
    async fn merchant_lookup_returns_scope_and_rejects_ambiguous_accounts() {
        let pool = provider_account_test_pool().await;
        insert_provider_account(
            &pool,
            "account-a",
            "tenant-a",
            Some("org-a"),
            "merchant-shared",
        )
        .await;

        let account =
            load_active_provider_account_by_merchant_id_sqlite(&pool, "stripe", "merchant-shared")
                .await
                .expect("merchant lookup")
                .expect("provider account");
        assert_eq!(account.tenant_id, "tenant-a");
        assert_eq!(account.organization_id.as_deref(), Some("org-a"));

        insert_provider_account(
            &pool,
            "account-b",
            "tenant-b",
            Some("org-b"),
            "merchant-shared",
        )
        .await;
        let error =
            load_active_provider_account_by_merchant_id_sqlite(&pool, "stripe", "merchant-shared")
                .await
                .expect_err("ambiguous merchant identity must fail closed");
        assert_eq!(error.code(), "conflict");
    }

    async fn provider_account_test_pool() -> sqlx::SqlitePool {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("sqlite memory pool");
        sqlx::query(
            r#"
            CREATE TABLE commerce_payment_provider_account (
                id TEXT PRIMARY KEY,
                tenant_id TEXT NOT NULL,
                organization_id TEXT,
                account_no TEXT NOT NULL,
                provider_code TEXT NOT NULL,
                merchant_id TEXT,
                environment TEXT NOT NULL,
                secret_ref TEXT NOT NULL,
                webhook_secret_ref TEXT,
                certificate_ref TEXT,
                status TEXT NOT NULL,
                metadata TEXT NOT NULL DEFAULT '{}',
                updated_at TEXT NOT NULL DEFAULT '2026-07-12T00:00:00Z',
                deleted_at TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .expect("create provider account test table");
        pool
    }

    async fn insert_provider_account(
        pool: &sqlx::SqlitePool,
        id: &str,
        tenant_id: &str,
        organization_id: Option<&str>,
        merchant_id: &str,
    ) {
        sqlx::query(
            r#"
            INSERT INTO commerce_payment_provider_account
                (id, tenant_id, organization_id, account_no, provider_code, merchant_id,
                 environment, secret_ref, status)
            VALUES (?, ?, ?, ?, 'stripe', ?, 'production', ?, 'active')
            "#,
        )
        .bind(id)
        .bind(tenant_id)
        .bind(organization_id)
        .bind(format!("NO-{id}"))
        .bind(merchant_id)
        .bind(format!("secret://{id}"))
        .execute(pool)
        .await
        .expect("insert provider account");
    }
}
