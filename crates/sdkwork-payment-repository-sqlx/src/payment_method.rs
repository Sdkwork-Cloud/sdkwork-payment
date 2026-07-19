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

// Catalog methods with configured channels are eligible only while at least
// one channel and its provider account are active. A channel without an account
// explicitly opts into deployment-level provider credentials.
const PROVIDER_ELIGIBILITY_FILTER: &str = r#"
  AND (
    NOT EXISTS (
      SELECT 1
      FROM commerce_payment_channel c0
      WHERE c0.tenant_id = m.tenant_id
        AND (
              c0.organization_id IS NULL
              OR m.organization_id IS NULL
              OR c0.organization_id = m.organization_id
            )
        AND (c0.method_id = m.id OR (c0.method_id IS NULL AND c0.provider_code = m.provider_code))
        AND c0.deleted_at IS NULL
    )
    OR EXISTS (
      SELECT 1
      FROM commerce_payment_channel c
      LEFT JOIN commerce_payment_provider_account a
        ON a.id = c.provider_account_id
       AND a.deleted_at IS NULL
      WHERE c.tenant_id = m.tenant_id
        AND (
              c.organization_id IS NULL
              OR m.organization_id IS NULL
              OR c.organization_id = m.organization_id
            )
        AND (c.method_id = m.id OR (c.method_id IS NULL AND c.provider_code = m.provider_code))
        AND c.status = 'active'
        AND c.deleted_at IS NULL
        AND (
              c.provider_account_id IS NULL
              OR (
                a.status = 'active'
                AND LOWER(a.provider_code) = LOWER(m.provider_code)
                AND a.tenant_id = m.tenant_id
                AND (
                      a.organization_id IS NULL
                      OR m.organization_id IS NULL
                      OR a.organization_id = m.organization_id
                    )
              )
            )
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
            "{LIST_PAYMENT_METHODS_BASE_SQLITE}{PROVIDER_ELIGIBILITY_FILTER}{SCENE_FILTER_SQLITE}
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
            "{LIST_PAYMENT_METHODS_BASE_POSTGRES}{PROVIDER_ELIGIBILITY_FILTER}{SCENE_FILTER_POSTGRES}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn active_catalog_method_is_gated_by_channel_and_provider_account() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        sqlx::query(
            "CREATE TABLE commerce_payment_method (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, method_key TEXT NOT NULL, display_name TEXT NOT NULL, provider_code TEXT NOT NULL, status TEXT NOT NULL, sort_order INTEGER NOT NULL, deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("method table");
        sqlx::query(
            "CREATE TABLE commerce_payment_channel (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, method_id TEXT, provider_code TEXT NOT NULL, scene_code TEXT NOT NULL, status TEXT NOT NULL, deleted_at TEXT, provider_account_id TEXT)",
        )
        .execute(&pool)
        .await
        .expect("channel table");
        sqlx::query(
            "CREATE TABLE commerce_payment_provider_account (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, provider_code TEXT NOT NULL, status TEXT NOT NULL, deleted_at TEXT)",
        )
        .execute(&pool)
        .await
        .expect("provider account table");
        sqlx::query(
            "INSERT INTO commerce_payment_method VALUES ('method-wechat-native','tenant-1','org-1','wechat_native','WeChat Pay Native','wechat_pay','active',300,NULL)",
        )
        .execute(&pool)
        .await
        .expect("method row");
        sqlx::query(
            "INSERT INTO commerce_payment_provider_account VALUES ('account-wechat','tenant-1','org-1','wechat_pay','inactive',NULL)",
        )
        .execute(&pool)
        .await
        .expect("provider row");
        sqlx::query(
            "INSERT INTO commerce_payment_channel VALUES ('channel-wechat','tenant-1','org-1','method-wechat-native','wechat_pay','api','active',NULL,'account-wechat')",
        )
        .execute(&pool)
        .await
        .expect("channel row");

        let store = SqliteCommercePaymentMethodStore::new(pool.clone());
        let query = PaymentMethodListQuery {
            tenant_id: "tenant-1".to_owned(),
            organization_id: Some("org-1".to_owned()),
            scene_code_filter: None,
            offset: 0,
            limit: 20,
        };
        assert!(store
            .list_payment_methods(query.clone())
            .await
            .expect("inactive provider list")
            .items
            .is_empty());

        sqlx::query("UPDATE commerce_payment_provider_account SET status = 'active'")
            .execute(&pool)
            .await
            .expect("activate provider");
        assert_eq!(
            store
                .list_payment_methods(query.clone())
                .await
                .expect("active provider list")
                .items
                .len(),
            1
        );

        sqlx::query("UPDATE commerce_payment_channel SET status = 'inactive'")
            .execute(&pool)
            .await
            .expect("deactivate channel");
        assert!(store
            .list_payment_methods(query)
            .await
            .expect("inactive channel list")
            .items
            .is_empty());
    }
}
