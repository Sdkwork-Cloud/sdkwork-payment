use sdkwork_contract_service::CommerceServiceError;
use sqlx::{Postgres, Row, Sqlite, Transaction};

use crate::shared::{store_error, string_cell};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaymentChannelSelection {
    pub channel_id: Option<String>,
    pub provider_account_id: Option<String>,
    pub provider_code: String,
}

pub(crate) async fn select_payment_channel_sqlite(
    tx: &mut Transaction<'_, Sqlite>,
    tenant_id: &str,
    organization_id: Option<&str>,
    payment_method: &str,
    currency_code: &str,
    amount: &str,
    payment_scene: Option<&str>,
) -> Result<PaymentChannelSelection, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        WITH selected_method AS (
            SELECT id, provider_code
            FROM commerce_payment_method
            WHERE method_key = CAST(? AS TEXT)
              AND status = 'active'
              AND tenant_id = CAST(? AS TEXT)
              AND (organization_id = CAST(? AS TEXT) OR organization_id IS NULL)
              AND deleted_at IS NULL
            ORDER BY CASE WHEN organization_id = CAST(? AS TEXT) THEN 0 ELSE 1 END,
                     sort_order ASC,
                     id ASC
            LIMIT 1
        ),
        eligible_channel AS (
            SELECT c.id AS channel_id,
                   c.provider_account_id,
                   c.provider_code,
                   CASE WHEN CAST(? AS TEXT) IS NOT NULL AND c.scene_code = CAST(? AS TEXT)
                        THEN 0 ELSE 1 END AS scene_rank,
                   COALESCE((
                       SELECT MIN(rr.priority)
                       FROM commerce_payment_route_rule rr
                       WHERE rr.tenant_id = c.tenant_id
                         AND (rr.organization_id = CAST(? AS TEXT) OR rr.organization_id IS NULL)
                         AND rr.channel_id = c.id
                         AND rr.status = 'active'
                         AND rr.deleted_at IS NULL
                         AND (rr.currency_code IS NULL OR UPPER(rr.currency_code) = UPPER(CAST(? AS TEXT)))
                         AND (rr.client_platform IS NULL OR LOWER(rr.client_platform) = LOWER(CAST(? AS TEXT)))
                         AND rr.purchase_type IS NULL
                         AND rr.country_code IS NULL
                         AND rr.user_segment IS NULL
                         AND rr.risk_level IS NULL
                         AND (rr.amount_min IS NULL OR CAST(rr.amount_min AS NUMERIC) <= CAST(? AS NUMERIC))
                         AND (rr.amount_max IS NULL OR CAST(rr.amount_max AS NUMERIC) >= CAST(? AS NUMERIC))
                         AND (rr.starts_at IS NULL OR datetime(rr.starts_at) <= CURRENT_TIMESTAMP)
                         AND (rr.ends_at IS NULL OR datetime(rr.ends_at) > CURRENT_TIMESTAMP)
                   ), 2147483647) AS route_priority,
                   c.priority,
                   c.sort_order
            FROM selected_method m
            INNER JOIN commerce_payment_channel c
              ON c.tenant_id = CAST(? AS TEXT)
             AND (c.method_id = m.id OR (c.method_id IS NULL AND LOWER(c.provider_code) = LOWER(m.provider_code)))
            LEFT JOIN commerce_payment_provider_account a
              ON a.id = c.provider_account_id
             AND a.deleted_at IS NULL
            WHERE c.status = 'active'
              AND c.deleted_at IS NULL
              AND LOWER(c.provider_code) = LOWER(m.provider_code)
              AND UPPER(c.currency_code) = UPPER(CAST(? AS TEXT))
              AND (c.organization_id = CAST(? AS TEXT) OR c.organization_id IS NULL)
              AND (c.provider_account_id IS NULL OR (
                    a.status = 'active'
                AND a.tenant_id = c.tenant_id
                AND LOWER(a.provider_code) = LOWER(c.provider_code)
                AND (a.organization_id = CAST(? AS TEXT) OR a.organization_id IS NULL)
              ))
        ),
        fallback AS (
            SELECT NULL AS channel_id,
                   NULL AS provider_account_id,
                   m.provider_code,
                   1 AS scene_rank,
                   2147483647 AS route_priority,
                   2147483647 AS priority,
                   2147483647 AS sort_order
            FROM selected_method m
            WHERE NOT EXISTS (
                SELECT 1
                FROM commerce_payment_channel c0
                WHERE c0.tenant_id = CAST(? AS TEXT)
                  AND (c0.method_id = m.id OR (c0.method_id IS NULL AND LOWER(c0.provider_code) = LOWER(m.provider_code)))
                  AND c0.deleted_at IS NULL
            )
        )
        SELECT channel_id, provider_account_id, provider_code
        FROM (
            SELECT * FROM eligible_channel
            UNION ALL
            SELECT * FROM fallback
        ) candidates
        ORDER BY CASE WHEN route_priority < 2147483647 THEN 0 ELSE 1 END,
                 route_priority ASC,
                 scene_rank ASC,
                 priority ASC,
                 sort_order ASC,
                 channel_id ASC
        LIMIT 1
        "#,
    )
    .bind(payment_method)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(organization_id)
    .bind(payment_scene)
    .bind(payment_scene)
    .bind(organization_id)
    .bind(currency_code)
    .bind(payment_scene)
    .bind(amount)
    .bind(amount)
    .bind(tenant_id)
    .bind(currency_code)
    .bind(organization_id)
    .bind(organization_id)
    .bind(tenant_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to select payment channel", error))?;

    row.map(|row| PaymentChannelSelection {
        channel_id: row.try_get("channel_id").ok().flatten(),
        provider_account_id: row.try_get("provider_account_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    })
    .ok_or_else(|| CommerceServiceError::conflict("payment method has no eligible channel"))
}

pub(crate) async fn select_payment_channel_postgres(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
    payment_method: &str,
    currency_code: &str,
    amount: &str,
    payment_scene: Option<&str>,
) -> Result<PaymentChannelSelection, CommerceServiceError> {
    let row = sqlx::query(
        r#"
        WITH selected_method AS (
            SELECT id, provider_code
            FROM commerce_payment_method
            WHERE method_key = CAST($1 AS TEXT)
              AND status = 'active'
              AND tenant_id = CAST($2 AS TEXT)
              AND (organization_id = CAST($3 AS TEXT) OR organization_id IS NULL)
              AND deleted_at IS NULL
            ORDER BY CASE WHEN organization_id = CAST($3 AS TEXT) THEN 0 ELSE 1 END,
                     sort_order ASC,
                     id ASC
            LIMIT 1
        ),
        eligible_channel AS (
            SELECT c.id AS channel_id,
                   c.provider_account_id,
                   c.provider_code,
                   CASE WHEN CAST($6 AS TEXT) IS NOT NULL AND c.scene_code = CAST($6 AS TEXT)
                        THEN 0 ELSE 1 END AS scene_rank,
                   COALESCE((
                       SELECT MIN(rr.priority)
                       FROM commerce_payment_route_rule rr
                       WHERE rr.tenant_id = c.tenant_id
                         AND (rr.organization_id = CAST($3 AS TEXT) OR rr.organization_id IS NULL)
                         AND rr.channel_id = c.id
                         AND rr.status = 'active'
                         AND rr.deleted_at IS NULL
                         AND (rr.currency_code IS NULL OR UPPER(rr.currency_code) = UPPER(CAST($4 AS TEXT)))
                         AND (rr.client_platform IS NULL OR LOWER(rr.client_platform) = LOWER(CAST($6 AS TEXT)))
                         AND rr.purchase_type IS NULL
                         AND rr.country_code IS NULL
                         AND rr.user_segment IS NULL
                         AND rr.risk_level IS NULL
                         AND (rr.amount_min IS NULL OR CAST(rr.amount_min AS NUMERIC) <= CAST($5 AS NUMERIC))
                         AND (rr.amount_max IS NULL OR CAST(rr.amount_max AS NUMERIC) >= CAST($5 AS NUMERIC))
                         AND (rr.starts_at IS NULL OR rr.starts_at <= CURRENT_TIMESTAMP)
                         AND (rr.ends_at IS NULL OR rr.ends_at > CURRENT_TIMESTAMP)
                   ), 2147483647) AS route_priority,
                   c.priority,
                   c.sort_order
            FROM selected_method m
            INNER JOIN commerce_payment_channel c
              ON c.tenant_id = CAST($2 AS TEXT)
             AND (c.method_id = m.id OR (c.method_id IS NULL AND LOWER(c.provider_code) = LOWER(m.provider_code)))
            LEFT JOIN commerce_payment_provider_account a
              ON a.id = c.provider_account_id
             AND a.deleted_at IS NULL
            WHERE c.status = 'active'
              AND c.deleted_at IS NULL
              AND LOWER(c.provider_code) = LOWER(m.provider_code)
              AND UPPER(c.currency_code) = UPPER(CAST($4 AS TEXT))
              AND (c.organization_id = CAST($3 AS TEXT) OR c.organization_id IS NULL)
              AND (c.provider_account_id IS NULL OR (
                    a.status = 'active'
                AND a.tenant_id = c.tenant_id
                AND LOWER(a.provider_code) = LOWER(c.provider_code)
                AND (a.organization_id = CAST($3 AS TEXT) OR a.organization_id IS NULL)
              ))
        ),
        fallback AS (
            SELECT NULL::text AS channel_id,
                   NULL::text AS provider_account_id,
                   m.provider_code,
                   1 AS scene_rank,
                   2147483647 AS route_priority,
                   2147483647 AS priority,
                   2147483647 AS sort_order
            FROM selected_method m
            WHERE NOT EXISTS (
                SELECT 1
                FROM commerce_payment_channel c0
                WHERE c0.tenant_id = CAST($2 AS TEXT)
                  AND (c0.method_id = m.id OR (c0.method_id IS NULL AND LOWER(c0.provider_code) = LOWER(m.provider_code)))
                  AND c0.deleted_at IS NULL
            )
        )
        SELECT channel_id, provider_account_id, provider_code
        FROM (
            SELECT * FROM eligible_channel
            UNION ALL
            SELECT * FROM fallback
        ) candidates
        ORDER BY CASE WHEN route_priority < 2147483647 THEN 0 ELSE 1 END,
                 route_priority ASC,
                 scene_rank ASC,
                 priority ASC,
                 sort_order ASC,
                 channel_id ASC NULLS LAST
        LIMIT 1
        "#,
    )
    .bind(payment_method)
    .bind(tenant_id)
    .bind(organization_id)
    .bind(currency_code)
    .bind(amount)
    .bind(payment_scene)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| store_error("failed to select payment channel", error))?;

    row.map(|row| PaymentChannelSelection {
        channel_id: row.try_get("channel_id").ok().flatten(),
        provider_account_id: row.try_get("provider_account_id").ok().flatten(),
        provider_code: string_cell(&row, "provider_code"),
    })
    .ok_or_else(|| CommerceServiceError::conflict("payment method has no eligible channel"))
}

#[cfg(test)]
mod tests {
    use super::select_payment_channel_sqlite;

    #[tokio::test]
    async fn route_rules_select_a_bound_account_and_fail_closed_when_channels_are_unavailable() {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:")
            .await
            .expect("sqlite pool");
        for ddl in [
            "CREATE TABLE commerce_payment_method (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, method_key TEXT NOT NULL, provider_code TEXT NOT NULL, status TEXT NOT NULL, sort_order INTEGER NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_provider_account (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, provider_code TEXT NOT NULL, status TEXT NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_channel (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, provider_account_id TEXT, method_id TEXT, provider_code TEXT NOT NULL, scene_code TEXT NOT NULL, currency_code TEXT NOT NULL, status TEXT NOT NULL, priority INTEGER NOT NULL, sort_order INTEGER NOT NULL, deleted_at TEXT)",
            "CREATE TABLE commerce_payment_route_rule (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, organization_id TEXT, priority INTEGER NOT NULL, purchase_type TEXT, country_code TEXT, currency_code TEXT, client_platform TEXT, amount_min TEXT, amount_max TEXT, user_segment TEXT, risk_level TEXT, channel_id TEXT NOT NULL, status TEXT NOT NULL, starts_at TEXT, ends_at TEXT, deleted_at TEXT)",
        ] {
            sqlx::query(ddl).execute(&pool).await.expect("create table");
        }
        sqlx::query("INSERT INTO commerce_payment_method VALUES ('method-card','tenant-1','org-1','stripe_card','stripe','active',0,NULL), ('method-legacy','tenant-1','org-1','legacy_pay','sandbox','active',1,NULL)")
            .execute(&pool).await.expect("insert methods");
        sqlx::query("INSERT INTO commerce_payment_provider_account VALUES ('account-a','tenant-1','org-1','stripe','active',NULL), ('account-b','tenant-1','org-1','stripe','active',NULL)")
            .execute(&pool).await.expect("insert accounts");
        sqlx::query("INSERT INTO commerce_payment_channel VALUES ('channel-a','tenant-1','org-1','account-a','method-card','stripe','web','CNY','active',10,10,NULL), ('channel-b','tenant-1','org-1','account-b','method-card','stripe','web','CNY','active',100,100,NULL)")
            .execute(&pool).await.expect("insert channels");
        sqlx::query("INSERT INTO commerce_payment_route_rule VALUES ('rule-b','tenant-1','org-1',1,NULL,NULL,'CNY','web','100','1000',NULL,NULL,'channel-b','active',NULL,NULL,NULL)")
            .execute(&pool).await.expect("insert route rule");

        let mut tx = pool.begin().await.expect("begin route transaction");
        let selected = select_payment_channel_sqlite(
            &mut tx,
            "tenant-1",
            Some("org-1"),
            "stripe_card",
            "CNY",
            "500",
            Some("web"),
        )
        .await
        .expect("select routed channel");
        tx.rollback().await.expect("rollback");
        assert_eq!(selected.channel_id.as_deref(), Some("channel-b"));
        assert_eq!(selected.provider_account_id.as_deref(), Some("account-b"));

        sqlx::query("UPDATE commerce_payment_provider_account SET status = 'inactive' WHERE id = 'account-b'")
            .execute(&pool).await.expect("disable routed account");
        let mut tx = pool.begin().await.expect("begin fallback transaction");
        let selected = select_payment_channel_sqlite(
            &mut tx,
            "tenant-1",
            Some("org-1"),
            "stripe_card",
            "CNY",
            "500",
            Some("web"),
        )
        .await
        .expect("select fallback channel");
        tx.rollback().await.expect("rollback");
        assert_eq!(selected.channel_id.as_deref(), Some("channel-a"));

        sqlx::query("UPDATE commerce_payment_provider_account SET status = 'inactive'")
            .execute(&pool)
            .await
            .expect("disable every account");
        let mut tx = pool.begin().await.expect("begin unavailable transaction");
        let error = select_payment_channel_sqlite(
            &mut tx,
            "tenant-1",
            Some("org-1"),
            "stripe_card",
            "CNY",
            "500",
            Some("web"),
        )
        .await
        .expect_err("configured but unavailable channels must fail closed");
        tx.rollback().await.expect("rollback");
        assert_eq!(error.code(), "conflict");

        let mut tx = pool.begin().await.expect("begin legacy transaction");
        let selected = select_payment_channel_sqlite(
            &mut tx,
            "tenant-1",
            Some("org-1"),
            "legacy_pay",
            "CNY",
            "500",
            None,
        )
        .await
        .expect("legacy deployment credentials fallback");
        tx.rollback().await.expect("rollback");
        assert_eq!(selected.channel_id, None);
        assert_eq!(selected.provider_account_id, None);
        assert_eq!(selected.provider_code, "sandbox");
    }
}
