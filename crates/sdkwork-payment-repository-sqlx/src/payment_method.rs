use sdkwork_contract_service::CommerceServiceError;
use sdkwork_payment_service::{
    parse_scene_codes_csv, PaymentMethodItem, PaymentMethodListPage, PaymentMethodListQuery,
};
use sqlx::postgres::PgRow;
use sqlx::sqlite::SqliteRow;
use sqlx::{Pool, Postgres, Row, Sqlite};

use crate::shared::{store_error, string_cell, StringCellRow};

const SCENE_FILTER_SQLITE: &str = r#"
  AND (
    ?3 IS NULL
    OR EXISTS (
      SELECT 1
      FROM commerce_payment_channel c
      WHERE c.tenant_id = m.tenant_id
        AND (
              c.organization_id IS NULL
              OR m.organization_id IS NULL
              OR c.organization_id = m.organization_id
            )
        AND (
              c.method_id = m.id
              OR (c.method_id IS NULL AND c.provider_code = m.provider_code)
            )
        AND c.status = 'active'
        AND c.deleted_at IS NULL
        AND c.scene_code = ?3
    )
  )
"#;

const SCENE_FILTER_POSTGRES: &str = r#"
  AND (
    $3 IS NULL
    OR EXISTS (
      SELECT 1
      FROM commerce_payment_channel c
      WHERE c.tenant_id = m.tenant_id
        AND (
              c.organization_id IS NULL
              OR m.organization_id IS NULL
              OR c.organization_id = m.organization_id
            )
        AND (
              c.method_id = m.id
              OR (c.method_id IS NULL AND c.provider_code = m.provider_code)
            )
        AND c.status = 'active'
        AND c.deleted_at IS NULL
        AND c.scene_code = $3
    )
  )
"#;

const LIST_PAYMENT_METHODS_BASE_SQLITE: &str = r#"
SELECT
    m.id,
    m.method_key,
    m.display_name,
    m.provider_code,
    m.sort_order,
    COALESCE((
        SELECT GROUP_CONCAT(DISTINCT c.scene_code)
        FROM commerce_payment_channel c
        WHERE c.tenant_id = m.tenant_id
          AND (
                c.organization_id IS NULL
                OR m.organization_id IS NULL
                OR c.organization_id = m.organization_id
              )
          AND (
                c.method_id = m.id
                OR (c.method_id IS NULL AND c.provider_code = m.provider_code)
              )
          AND c.status = 'active'
          AND c.deleted_at IS NULL
    ), 'web') AS scene_codes,
    COUNT(*) OVER() AS total_count
FROM commerce_payment_method m
WHERE (
        (m.tenant_id = CAST(?1 AS TEXT) AND m.organization_id = CAST(?2 AS TEXT))
        OR (m.tenant_id = CAST(?1 AS TEXT) AND m.organization_id IS NULL)
      )
  AND m.status = 'active'
  AND m.deleted_at IS NULL
"#;

const LIST_PAYMENT_METHODS_BASE_POSTGRES: &str = r#"
SELECT
    m.id,
    m.method_key,
    m.display_name,
    m.provider_code,
    m.sort_order,
    COALESCE((
        SELECT STRING_AGG(DISTINCT c.scene_code, ',')
        FROM commerce_payment_channel c
        WHERE c.tenant_id = m.tenant_id
          AND (
                c.organization_id IS NULL
                OR m.organization_id IS NULL
                OR c.organization_id = m.organization_id
              )
          AND (
                c.method_id = m.id
                OR (c.method_id IS NULL AND c.provider_code = m.provider_code)
              )
          AND c.status = 'active'
          AND c.deleted_at IS NULL
    ), 'web') AS scene_codes,
    COUNT(*) OVER() AS total_count
FROM commerce_payment_method m
WHERE (
        (m.tenant_id = CAST($1 AS TEXT) AND m.organization_id = CAST($2 AS TEXT))
        OR (m.tenant_id = CAST($1 AS TEXT) AND m.organization_id IS NULL)
      )
  AND m.status = 'active'
  AND m.deleted_at IS NULL
"#;

#[derive(Debug, Clone)]
pub struct SqliteCommercePaymentMethodStore {
    pool: Pool<Sqlite>,
}

impl SqliteCommercePaymentMethodStore {
    pub fn new(pool: Pool<Sqlite>) -> Self {
        Self { pool }
    }

    pub async fn list_payment_methods(
        &self,
        query: PaymentMethodListQuery,
    ) -> Result<PaymentMethodListPage, CommerceServiceError> {
        let sql = format!(
            "{LIST_PAYMENT_METHODS_BASE_SQLITE}{SCENE_FILTER_SQLITE}
ORDER BY COALESCE(m.sort_order, 0) ASC, m.id ASC
LIMIT ?4 OFFSET ?5"
        );
        let rows = sqlx::query(&sql)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.scene_code_filter.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| store_error("failed to list payment methods", error))?;

        let total_items = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows.iter().map(map_sqlite_payment_method_row).collect();

        Ok(PaymentMethodListPage { items, total_items })
    }
}

#[derive(Debug, Clone)]
pub struct PostgresCommercePaymentMethodStore {
    pool: Pool<Postgres>,
}

impl PostgresCommercePaymentMethodStore {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    pub async fn list_payment_methods(
        &self,
        query: PaymentMethodListQuery,
    ) -> Result<PaymentMethodListPage, CommerceServiceError> {
        let sql = format!(
            "{LIST_PAYMENT_METHODS_BASE_POSTGRES}{SCENE_FILTER_POSTGRES}
ORDER BY COALESCE(m.sort_order, 0) ASC, m.id ASC
LIMIT $4 OFFSET $5"
        );
        let rows = sqlx::query(&sql)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(query.scene_code_filter.as_deref())
            .bind(query.limit)
            .bind(query.offset)
            .fetch_all(&self.pool)
            .await
            .map_err(|error| store_error("failed to list payment methods", error))?;

        let total_items = rows
            .first()
            .and_then(|row| row.try_get::<i64, _>("total_count").ok())
            .unwrap_or(0);
        let items = rows.iter().map(map_postgres_payment_method_row).collect();

        Ok(PaymentMethodListPage { items, total_items })
    }
}

fn map_sqlite_payment_method_row(row: &SqliteRow) -> PaymentMethodItem {
    map_payment_method_row(row, row.try_get::<i64, _>("sort_order").unwrap_or(0))
}

fn map_postgres_payment_method_row(row: &PgRow) -> PaymentMethodItem {
    map_payment_method_row(row, row.try_get::<i64, _>("sort_order").unwrap_or(0))
}

fn map_payment_method_row(row: &impl StringCellRow, sort_order: i64) -> PaymentMethodItem {
    PaymentMethodItem {
        id: string_cell(row, "id"),
        method_key: string_cell(row, "method_key"),
        display_name: string_cell(row, "display_name"),
        provider_code: string_cell(row, "provider_code"),
        scene_codes: parse_scene_codes_csv(&string_cell(row, "scene_codes")),
        sort_order,
    }
}
