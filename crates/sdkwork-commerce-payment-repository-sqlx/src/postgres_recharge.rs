use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use sdkwork_commerce_contract_service::{
    CommerceMoney, CommercePaymentStatus, CommerceServiceError,
};
use sdkwork_commerce_payment_service::{
    CheckoutStatusQuery, CheckoutStatusSnapshot, CreatePointsRechargeOrderCommand,
    CreatePointsRechargeOrderOutcome, RechargeGrantPreview, RechargePackageItem,
    RechargePackageListQuery, RechargeSettingsQuery, RechargeSettingsSnapshot,
};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Row, Transaction};

const DEFAULT_BASE_CURRENCY_CODE: &str = "CNY";
const DEFAULT_BASE_POINTS_PER_CNY: &str = "10";
const DEFAULT_USD_TO_CNY_RATE: &str = "7";
const RECHARGE_RULE_NO: &str = "CASH_TO_POINTS";

const LOAD_RECHARGE_PACKAGES_SCOPED: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS id,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points
FROM commerce_recharge_package p
LEFT JOIN commerce_product_sku s
    ON s.id = p.sku_id
   AND s.status = 'active'
LEFT JOIN commerce_product_spu pr
    ON pr.id = s.spu_id
   AND pr.status = 'active'
WHERE (
        (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT))
        OR (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL)
      )
  AND p.status = 'active'
  AND (p.valid_from IS NULL OR p.valid_from <= $3)
  AND (p.valid_to IS NULL OR p.valid_to >= $3)
GROUP BY p.id, p.tenant_id, p.organization_id, p.price_amount, p.currency_code, p.bonus_points, p.sort_weight
ORDER BY
    CASE
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT) THEN 0
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    COALESCE(p.sort_weight, 0) ASC,
    p.currency_code ASC,
    p.price_amount ASC,
    p.id ASC
LIMIT 100
"#;

const LOAD_RECHARGE_PACKAGES_PUBLIC: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS id,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points
FROM commerce_recharge_package p
LEFT JOIN commerce_product_sku s
    ON s.id = p.sku_id
   AND s.status = 'active'
LEFT JOIN commerce_product_spu pr
    ON pr.id = s.spu_id
   AND pr.status = 'active'
WHERE p.tenant_id = '100001'
  AND (p.organization_id = '0' OR p.organization_id IS NULL)
  AND p.status = 'active'
  AND (p.valid_from IS NULL OR p.valid_from <= $1)
  AND (p.valid_to IS NULL OR p.valid_to >= $1)
GROUP BY p.id, p.price_amount, p.currency_code, p.bonus_points, p.sort_weight
ORDER BY
    COALESCE(p.sort_weight, 0) ASC,
    p.currency_code ASC,
    p.price_amount ASC,
    p.id ASC
LIMIT 100
"#;

const LOAD_RECHARGE_SETTINGS_SCOPED: &str = r#"
SELECT
    CAST(rate AS TEXT) AS rate,
    CAST(COALESCE(remark, '') AS TEXT) AS remark
FROM commerce_exchange_rule
WHERE (
        (tenant_id = CAST($1 AS TEXT) AND organization_id = CAST($2 AS TEXT))
        OR (tenant_id = CAST($1 AS TEXT) AND organization_id IS NULL)
      )
  AND LOWER(source_asset_type) = 'cash'
  AND LOWER(target_asset_type) = 'points'
  AND status = 'active'
ORDER BY
    CASE
        WHEN tenant_id = CAST($1 AS TEXT) AND organization_id = CAST($2 AS TEXT) THEN 0
        WHEN tenant_id = CAST($1 AS TEXT) AND organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    CASE
        WHEN rule_no = $3 THEN 0
        ELSE 1
    END ASC,
    id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_SETTINGS_PUBLIC: &str = r#"
SELECT
    CAST(rate AS TEXT) AS rate,
    CAST(COALESCE(remark, '') AS TEXT) AS remark
FROM commerce_exchange_rule
WHERE LOWER(source_asset_type) = 'cash'
  AND tenant_id = '100001'
  AND (organization_id = '0' OR organization_id IS NULL)
  AND LOWER(target_asset_type) = 'points'
  AND status = 'active'
ORDER BY
    CASE
        WHEN rule_no = $1 THEN 0
        ELSE 1
    END ASC,
    id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PACK_BY_ID: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS package_id,
    COALESCE(NULLIF(p.name, ''), 'Points recharge package') AS name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points,
    CAST(p.sku_id AS TEXT) AS sku_id
FROM commerce_recharge_package p
WHERE (
        (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT))
        OR (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL)
      )
  AND p.status = 'active'
  AND p.id = $3
  AND (p.valid_from IS NULL OR p.valid_from <= $4)
  AND (p.valid_to IS NULL OR p.valid_to >= $4)
ORDER BY
    CASE
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT) THEN 0
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    COALESCE(p.sort_weight, 0) ASC,
    p.id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PACK_BY_ID_PUBLIC: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS package_id,
    COALESCE(NULLIF(p.name, ''), 'Points recharge package') AS name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points,
    CAST(p.sku_id AS TEXT) AS sku_id
FROM commerce_recharge_package p
WHERE p.tenant_id = '100001'
  AND (p.organization_id = '0' OR p.organization_id IS NULL)
  AND p.status = 'active'
  AND p.id = $1
  AND (p.valid_from IS NULL OR p.valid_from <= $2)
  AND (p.valid_to IS NULL OR p.valid_to >= $2)
ORDER BY COALESCE(p.sort_weight, 0) ASC, p.id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PACK_FOR_AMOUNT: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS package_id,
    COALESCE(NULLIF(p.name, ''), 'Points recharge package') AS name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points,
    CAST(p.sku_id AS TEXT) AS sku_id
FROM commerce_recharge_package p
WHERE (
        (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT))
        OR (p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL)
      )
  AND p.status = 'active'
  AND COALESCE(NULLIF(p.currency_code, ''), 'CNY') = $3
  AND CAST(p.price_amount AS TEXT) IN ($4, $5, $6)
  AND (p.valid_from IS NULL OR p.valid_from <= $7)
  AND (p.valid_to IS NULL OR p.valid_to >= $7)
ORDER BY
    CASE
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id = CAST($2 AS TEXT) THEN 0
        WHEN p.tenant_id = CAST($1 AS TEXT) AND p.organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    COALESCE(p.sort_weight, 0) ASC,
    p.id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PACK_FOR_AMOUNT_PUBLIC: &str = r#"
SELECT
    CAST(p.id AS TEXT) AS package_id,
    COALESCE(NULLIF(p.name, ''), 'Points recharge package') AS name,
    CAST(p.price_amount AS TEXT) AS price_amount,
    COALESCE(NULLIF(p.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(p.bonus_points, 0) AS TEXT) AS bonus_points,
    CAST(p.sku_id AS TEXT) AS sku_id
FROM commerce_recharge_package p
WHERE p.tenant_id = '100001'
  AND (p.organization_id = '0' OR p.organization_id IS NULL)
  AND p.status = 'active'
  AND COALESCE(NULLIF(p.currency_code, ''), 'CNY') = $1
  AND CAST(p.price_amount AS TEXT) IN ($2, $3, $4)
  AND (p.valid_from IS NULL OR p.valid_from <= $5)
  AND (p.valid_to IS NULL OR p.valid_to >= $5)
ORDER BY COALESCE(p.sort_weight, 0) ASC, p.id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_METHOD: &str = r#"
SELECT method_key, provider_code
FROM commerce_payment_method
WHERE (
        (tenant_id = CAST($1 AS TEXT) AND organization_id = CAST($2 AS TEXT))
        OR (tenant_id = CAST($1 AS TEXT) AND organization_id IS NULL)
        OR (tenant_id = '100001' AND (organization_id = '0' OR organization_id IS NULL))
      )
  AND status = 'active'
  AND LOWER(method_key) = $3
ORDER BY
    CASE
        WHEN tenant_id = CAST($1 AS TEXT) AND organization_id = CAST($2 AS TEXT) THEN 0
        WHEN tenant_id = CAST($1 AS TEXT) AND organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    COALESCE(sort_order, 0) ASC,
    id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PRODUCT_SKU_BY_ID: &str = r#"
SELECT
    CAST(s.id AS TEXT) AS sku_id,
    COALESCE(NULLIF(s.name, ''), NULLIF(s.title, ''), NULLIF(pr.title, ''), 'Points recharge') AS product_name
FROM commerce_product_sku s
JOIN commerce_product_spu pr ON pr.id = s.spu_id
WHERE s.id = $1
  AND s.status = 'active'
  AND pr.status = 'active'
LIMIT 1
"#;

const LOAD_RECHARGE_PRODUCT_SKU_FOR_AMOUNT: &str = r#"
SELECT
    CAST(s.id AS TEXT) AS sku_id,
    COALESCE(NULLIF(s.name, ''), NULLIF(s.title, ''), NULLIF(pr.title, ''), 'Points recharge') AS product_name
FROM commerce_product_sku s
JOIN commerce_product_spu pr ON pr.id = s.spu_id
WHERE (
        (
            s.tenant_id = CAST($1 AS TEXT)
            AND (s.organization_id = CAST($2 AS TEXT) OR s.organization_id IS NULL)
            AND pr.tenant_id = CAST($1 AS TEXT)
            AND (pr.organization_id = CAST($2 AS TEXT) OR pr.organization_id IS NULL)
        )
      )
  AND COALESCE(NULLIF(s.currency_code, ''), 'CNY') = $3
  AND s.status = 'active'
  AND pr.status = 'active'
ORDER BY
    CASE WHEN CAST(s.price_amount AS TEXT) IN ($4, $5, $6) THEN 0 ELSE 1 END,
    CASE
        WHEN s.tenant_id = CAST($1 AS TEXT) AND s.organization_id = CAST($2 AS TEXT) THEN 0
        WHEN s.tenant_id = CAST($1 AS TEXT) AND s.organization_id IS NULL THEN 1
        ELSE 2
    END ASC,
    pr.id ASC,
    s.id ASC
LIMIT 1
"#;

const LOAD_RECHARGE_PRODUCT_SKU_FOR_AMOUNT_PUBLIC: &str = r#"
SELECT
    CAST(s.id AS TEXT) AS sku_id,
    COALESCE(NULLIF(s.name, ''), NULLIF(s.title, ''), NULLIF(pr.title, ''), 'Points recharge') AS product_name
FROM commerce_product_sku s
JOIN commerce_product_spu pr ON pr.id = s.spu_id
WHERE s.tenant_id = '100001'
  AND (s.organization_id = '0' OR s.organization_id IS NULL)
  AND pr.tenant_id = '100001'
  AND (pr.organization_id = '0' OR pr.organization_id IS NULL)
  AND COALESCE(NULLIF(s.currency_code, ''), 'CNY') = $1
  AND s.status = 'active'
  AND pr.status = 'active'
ORDER BY
    CASE WHEN CAST(s.price_amount AS TEXT) IN ($2, $3, $4) THEN 0 ELSE 1 END,
    pr.id ASC,
    s.id ASC
LIMIT 1
"#;

const LOAD_CHECKOUT_STATUS: &str = r#"
SELECT
    o.id AS order_id,
    pi.id AS payment_id,
    pa.id AS payment_attempt_id,
    COALESCE(NULLIF(o.order_no, ''), NULLIF(pa.out_trade_no, ''), '-') AS order_no,
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, ''), '-') AS out_trade_no,
    CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) AS amount,
    COALESCE(NULLIF(pa.currency_code, ''), NULLIF(pi.currency_code, ''), NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(
        NULLIF(pa.callback_payload::jsonb ->> 'points', ''),
        '0'
    ) AS TEXT) AS points_value,
    COALESCE(NULLIF(pa.payment_method, ''), NULLIF(pi.payment_method, ''), '-') AS payment_method,
    COALESCE(NULLIF(pa.provider_code, ''), NULLIF(pi.provider_code, ''), '-') AS provider_code,
    o.status AS order_status,
    pi.status AS payment_status,
    pa.status AS payment_attempt_status,
    CAST(o.created_at AS TEXT) AS created_at,
    CAST(COALESCE(o.expired_at, '') AS TEXT) AS expires_at,
    CAST(COALESCE(pa.paid_at, o.paid_at, '') AS TEXT) AS paid_at
FROM commerce_order o
LEFT JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.order_id = o.id
LEFT JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND (
        o.order_no = $4
        OR pa.out_trade_no = $4
   )
ORDER BY COALESCE(pa.created_at, pi.created_at, o.created_at) DESC NULLS LAST, o.id DESC
LIMIT 1
"#;

const LOAD_REUSABLE_RECHARGE_CHECKOUT: &str = r#"
SELECT
    o.id AS order_id,
    pi.id AS payment_id,
    pa.id AS payment_attempt_id,
    COALESCE(NULLIF(o.order_no, ''), NULLIF(pa.out_trade_no, ''), '-') AS order_no,
    COALESCE(NULLIF(pa.out_trade_no, ''), NULLIF(o.order_no, ''), '-') AS out_trade_no,
    CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) AS amount,
    COALESCE(NULLIF(pa.currency_code, ''), NULLIF(pi.currency_code, ''), NULLIF(o.currency_code, ''), 'CNY') AS currency_code,
    CAST(COALESCE(
        NULLIF(pa.callback_payload::jsonb ->> 'points', ''),
        '0'
    ) AS TEXT) AS points_value,
    COALESCE(NULLIF(pa.payment_method, ''), NULLIF(pi.payment_method, ''), '-') AS payment_method,
    COALESCE(NULLIF(pa.provider_code, ''), NULLIF(pi.provider_code, ''), '-') AS provider_code,
    o.status AS order_status,
    pi.status AS payment_status,
    pa.status AS payment_attempt_status,
    CAST(o.created_at AS TEXT) AS created_at,
    CAST(COALESCE(o.expired_at, '') AS TEXT) AS expires_at,
    CAST(COALESCE(pa.paid_at, o.paid_at, '') AS TEXT) AS paid_at
FROM commerce_order o
JOIN commerce_payment_intent pi
    ON pi.tenant_id = o.tenant_id
   AND (pi.organization_id IS NULL OR o.organization_id IS NULL OR pi.organization_id = o.organization_id)
   AND pi.order_id = o.id
JOIN commerce_payment_attempt pa
    ON pa.tenant_id = o.tenant_id
   AND (pa.organization_id IS NULL OR o.organization_id IS NULL OR pa.organization_id = o.organization_id)
   AND pa.order_id = o.id
WHERE o.tenant_id = CAST($1 AS TEXT)
  AND ((o.organization_id = CAST($2 AS TEXT)) OR (o.organization_id IS NULL AND $2 IS NULL))
  AND o.owner_user_id = CAST($3 AS TEXT)
  AND o.subject = 'points_recharge'
  AND CAST(COALESCE(NULLIF(pa.amount, ''), NULLIF(pi.amount, ''), '0') AS TEXT) IN ($4, $5, $6)
  AND COALESCE(NULLIF(pa.currency_code, ''), NULLIF(pi.currency_code, ''), NULLIF(o.currency_code, ''), 'CNY') = $7
  AND CAST(COALESCE(
        NULLIF(pa.callback_payload::jsonb ->> 'points', ''),
        '0'
      ) AS BIGINT) = $8
  AND COALESCE(NULLIF(pa.callback_payload::jsonb ->> 'packageId', ''), '') = COALESCE(CAST($9 AS TEXT), '')
  AND LOWER(COALESCE(NULLIF(o.status, ''), 'pending_payment')) IN ('draft', 'pending', 'pending_payment')
  AND LOWER(COALESCE(NULLIF(pi.status, ''), 'pending')) IN ('created', 'pending', 'processing')
  AND LOWER(COALESCE(NULLIF(pa.status, ''), 'pending')) IN ('created', 'pending', 'processing')
  AND (o.expired_at IS NULL OR o.expired_at = '' OR o.expired_at > $10)
ORDER BY COALESCE(pa.created_at, pi.created_at, o.created_at) DESC NULLS LAST, o.id DESC
LIMIT 1
"#;

#[derive(Debug, Clone)]
pub struct PostgresCommerceRechargeStore {
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct RechargeMethod {
    method_key: String,
    provider_code: String,
    payment_product: String,
}

#[derive(Debug, Clone)]
struct RechargePack {
    id: String,
    name: String,
    price_amount: CommerceMoney,
    currency_code: String,
    bonus_points: i64,
    sku_id: String,
}

#[derive(Debug, Clone)]
struct RechargeProductSku {
    sku_id: String,
    product_name: String,
}

#[derive(Debug, Clone)]
struct RechargeSettingsModel {
    base_currency_code: String,
    base_points_per_cny: String,
    currency_to_cny_rates: BTreeMap<String, String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct RechargeSettingsRemark {
    #[serde(default)]
    base_currency_code: Option<String>,
    #[serde(default)]
    currency_to_cny_rates: BTreeMap<String, String>,
}

impl PostgresCommerceRechargeStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_recharge_packages(
        &self,
        query: RechargePackageListQuery,
    ) -> Result<Vec<RechargePackageItem>, CommerceServiceError> {
        let settings = self
            .load_recharge_settings_model(&query.tenant_id, query.organization_id.as_deref())
            .await?;
        let rows = if query.tenant_id.trim().is_empty() {
            sqlx::query(LOAD_RECHARGE_PACKAGES_PUBLIC)
                .bind(current_query_timestamp())
                .fetch_all(&self.pool)
                .await
        } else {
            let scoped_rows = sqlx::query(LOAD_RECHARGE_PACKAGES_SCOPED)
                .bind(&query.tenant_id)
                .bind(query.organization_id.as_deref())
                .bind(current_query_timestamp())
                .fetch_all(&self.pool)
                .await
                .map_err(|error| store_error("failed to list recharge packages", error))?;
            if scoped_rows.is_empty() {
                sqlx::query(LOAD_RECHARGE_PACKAGES_PUBLIC)
                    .bind(current_query_timestamp())
                    .fetch_all(&self.pool)
                    .await
            } else {
                Ok(scoped_rows)
            }
        }
        .map_err(|error| store_error("failed to list recharge packages", error))?;

        rows.iter()
            .map(|row| map_package_row(row, &settings))
            .collect()
    }

    pub async fn load_recharge_settings(
        &self,
        query: RechargeSettingsQuery,
    ) -> Result<RechargeSettingsSnapshot, CommerceServiceError> {
        let settings = self
            .load_recharge_settings_model(&query.tenant_id, query.organization_id.as_deref())
            .await?;
        let preview_examples = build_recharge_preview_examples(&settings)?;

        RechargeSettingsSnapshot::new(
            &settings.base_currency_code,
            &settings.base_points_per_cny,
            settings.currency_to_cny_rates,
            preview_examples,
        )
    }

    pub async fn create_points_recharge_order(
        &self,
        command: CreatePointsRechargeOrderCommand,
    ) -> Result<CreatePointsRechargeOrderOutcome, CommerceServiceError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|error| store_error("failed to begin recharge transaction", error))?;
        let settings = load_recharge_settings_for_transaction(
            &mut tx,
            &command.tenant_id,
            command.organization_id.as_deref(),
        )
        .await?;
        let pack = load_recharge_pack(&mut tx, &command).await?;
        let credited_points = compute_grant_amount(
            command.amount.as_str(),
            &command.currency_code,
            pack.as_ref().map(|item| item.bonus_points).unwrap_or(0),
            &settings,
        )?;
        if let Some(reusable_checkout_status) = load_reusable_recharge_checkout_status(
            &mut tx,
            &command,
            pack.as_ref(),
            credited_points,
        )
        .await?
        {
            tx.rollback().await.map_err(|error| {
                store_error("failed to rollback reusable recharge transaction", error)
            })?;
            return Ok(recharge_outcome_from_checkout_status(
                reusable_checkout_status,
            ));
        }
        let method = load_recharge_method(&mut tx, &command).await?;
        let product = load_recharge_product_sku(&mut tx, &command, pack.as_ref()).await?;
        let product_name = pack
            .as_ref()
            .map(|item| item.name.clone())
            .unwrap_or_else(|| product.product_name.clone());

        insert_order(&mut tx, &command).await?;
        insert_order_item(&mut tx, &command, &product, &product_name).await?;
        insert_order_amount_breakdown(&mut tx, &command).await?;
        insert_payment(&mut tx, &command, &method, credited_points).await?;
        insert_recharge_billing_history(&mut tx, &command, &method, credited_points).await?;
        tx.commit()
            .await
            .map_err(|error| store_error("failed to commit recharge transaction", error))?;
        let cashier_url = recharge_cashier_url(&command.order_no, &command.out_trade_no);

        Ok(CreatePointsRechargeOrderOutcome {
            success: true,
            order_no: command.order_no,
            out_trade_no: command.out_trade_no,
            amount: command.amount,
            currency_code: command.currency_code,
            points: credited_points,
            provider_code: method.provider_code.clone(),
            payment_method: method.method_key,
            payment_product: method.payment_product.clone(),
            status: "pending".to_string(),
            next_action: "scan_qr".to_string(),
            cashier_url: cashier_url.clone(),
            qr_code_payload: cashier_url,
            request_payment_payload: None,
        })
    }

    pub async fn load_checkout_status(
        &self,
        query: CheckoutStatusQuery,
    ) -> Result<Option<CheckoutStatusSnapshot>, CommerceServiceError> {
        let row = sqlx::query(LOAD_CHECKOUT_STATUS)
            .bind(&query.tenant_id)
            .bind(query.organization_id.as_deref())
            .bind(&query.owner_user_id)
            .bind(&query.order_no)
            .fetch_optional(&self.pool)
            .await
            .map_err(|error| store_error("failed to load checkout status", error))?;

        row.as_ref().map(map_checkout_status).transpose()
    }

    async fn load_recharge_settings_model(
        &self,
        tenant_id: &str,
        organization_id: Option<&str>,
    ) -> Result<RechargeSettingsModel, CommerceServiceError> {
        load_recharge_settings_from_pool(&self.pool, tenant_id, organization_id).await
    }
}

async fn load_recharge_settings_from_pool(
    pool: &PgPool,
    tenant_id: &str,
    organization_id: Option<&str>,
) -> Result<RechargeSettingsModel, CommerceServiceError> {
    let row = if tenant_id.trim().is_empty() {
        sqlx::query(LOAD_RECHARGE_SETTINGS_PUBLIC)
            .bind(RECHARGE_RULE_NO)
            .fetch_optional(pool)
            .await
    } else {
        let scoped_row = sqlx::query(LOAD_RECHARGE_SETTINGS_SCOPED)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(RECHARGE_RULE_NO)
            .fetch_optional(pool)
            .await
            .map_err(|error| store_error("failed to load recharge settings", error))?;
        if scoped_row.is_some() {
            Ok(scoped_row)
        } else {
            sqlx::query(LOAD_RECHARGE_SETTINGS_PUBLIC)
                .bind(RECHARGE_RULE_NO)
                .fetch_optional(pool)
                .await
        }
    }
    .map_err(|error| store_error("failed to load recharge settings", error))?;

    map_settings_row(row.as_ref())
}

async fn load_recharge_settings_for_transaction(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    organization_id: Option<&str>,
) -> Result<RechargeSettingsModel, CommerceServiceError> {
    let row = if tenant_id.trim().is_empty() {
        sqlx::query(LOAD_RECHARGE_SETTINGS_PUBLIC)
            .bind(RECHARGE_RULE_NO)
            .fetch_optional(&mut **tx)
            .await
    } else {
        let scoped_row = sqlx::query(LOAD_RECHARGE_SETTINGS_SCOPED)
            .bind(tenant_id)
            .bind(organization_id)
            .bind(RECHARGE_RULE_NO)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to load recharge settings", error))?;
        if scoped_row.is_some() {
            Ok(scoped_row)
        } else {
            sqlx::query(LOAD_RECHARGE_SETTINGS_PUBLIC)
                .bind(RECHARGE_RULE_NO)
                .fetch_optional(&mut **tx)
                .await
        }
    }
    .map_err(|error| store_error("failed to load recharge settings", error))?;
    map_settings_row(row.as_ref())
}

fn map_settings_row(
    row: Option<&sqlx::postgres::PgRow>,
) -> Result<RechargeSettingsModel, CommerceServiceError> {
    let base_points_per_cny = row
        .map(|row| string_cell(row, "rate"))
        .filter(|value| !value.trim().is_empty())
        .map(|value| normalize_decimal_string(&value))
        .unwrap_or_else(|| DEFAULT_BASE_POINTS_PER_CNY.to_string());
    let remark_json = row
        .map(|row| string_cell(row, "remark"))
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(default_recharge_remark_json);
    let remark = parse_recharge_settings_remark(&remark_json)?;
    let mut currency_to_cny_rates = remark.currency_to_cny_rates;
    if currency_to_cny_rates.is_empty() {
        currency_to_cny_rates = default_currency_to_cny_rates();
    }
    currency_to_cny_rates
        .entry(DEFAULT_BASE_CURRENCY_CODE.to_string())
        .or_insert_with(|| "1".to_string());
    let base_currency_code = remark
        .base_currency_code
        .unwrap_or_else(|| DEFAULT_BASE_CURRENCY_CODE.to_string())
        .trim()
        .to_ascii_uppercase();

    Ok(RechargeSettingsModel {
        base_currency_code,
        base_points_per_cny,
        currency_to_cny_rates,
    })
}

fn parse_recharge_settings_remark(
    json: &str,
) -> Result<RechargeSettingsRemark, CommerceServiceError> {
    serde_json::from_str::<RechargeSettingsRemark>(json).map_err(|error| {
        CommerceServiceError::storage(format!("invalid recharge settings remark json: {error}"))
    })
}

fn default_recharge_remark_json() -> String {
    serde_json::json!({
        "baseCurrencyCode": DEFAULT_BASE_CURRENCY_CODE,
        "currencyToCnyRates": default_currency_to_cny_rates(),
    })
    .to_string()
}

fn default_currency_to_cny_rates() -> BTreeMap<String, String> {
    BTreeMap::from([
        (DEFAULT_BASE_CURRENCY_CODE.to_string(), "1".to_string()),
        ("USD".to_string(), DEFAULT_USD_TO_CNY_RATE.to_string()),
    ])
}

fn build_recharge_preview_examples(
    settings: &RechargeSettingsModel,
) -> Result<BTreeMap<String, BTreeMap<String, RechargeGrantPreview>>, CommerceServiceError> {
    let mut preview_examples = BTreeMap::new();
    for currency_code in settings.currency_to_cny_rates.keys() {
        let mut examples = BTreeMap::new();
        for amount in ["5", "10", "20", "30", "50", "100", "200", "500", "1000"] {
            let grant_amount = compute_grant_amount(amount, currency_code, 0, settings)?;
            examples.insert(amount.to_string(), RechargeGrantPreview { grant_amount });
        }
        preview_examples.insert(currency_code.clone(), examples);
    }
    Ok(preview_examples)
}

fn map_package_row(
    row: &sqlx::postgres::PgRow,
    settings: &RechargeSettingsModel,
) -> Result<RechargePackageItem, CommerceServiceError> {
    let price_amount = commerce_money_cell(row, "price_amount", "recharge package price amount")?;
    let currency_code = string_cell(row, "currency_code")
        .trim()
        .to_ascii_uppercase();
    let bonus_points = required_non_negative_integer_cell(row, "bonus_points")?;
    let grant_amount = compute_grant_amount(
        price_amount.as_str(),
        &currency_code,
        bonus_points,
        settings,
    )?;
    RechargePackageItem::new(
        &string_cell(row, "id"),
        price_amount,
        &currency_code,
        bonus_points,
        grant_amount,
        grant_amount,
    )
}

async fn load_recharge_method(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
) -> Result<RechargeMethod, CommerceServiceError> {
    let requested_method = normalize_method_key(&command.method);
    let row = sqlx::query(LOAD_RECHARGE_METHOD)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&requested_method)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load recharge method", error))?
        .ok_or_else(|| CommerceServiceError::conflict("recharge payment method is unavailable"))?;
    let method_key = normalize_method_key(&string_cell(&row, "method_key"));
    let provider_code = normalize_method_key(&string_cell(&row, "provider_code"));
    let payment_product = recharge_payment_product(&method_key)?.to_string();
    Ok(RechargeMethod {
        method_key,
        provider_code,
        payment_product,
    })
}

async fn load_recharge_pack(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
) -> Result<Option<RechargePack>, CommerceServiceError> {
    if let Some(package_id) = command.package_id.as_deref() {
        let row = if command.tenant_id.trim().is_empty() {
            sqlx::query(LOAD_RECHARGE_PACK_BY_ID_PUBLIC)
                .bind(package_id)
                .bind(&command.requested_at)
                .fetch_optional(&mut **tx)
                .await
        } else {
            let scoped_row = sqlx::query(LOAD_RECHARGE_PACK_BY_ID)
                .bind(&command.tenant_id)
                .bind(command.organization_id.as_deref())
                .bind(package_id)
                .bind(&command.requested_at)
                .fetch_optional(&mut **tx)
                .await
                .map_err(|error| store_error("failed to load recharge package by id", error))?;
            if scoped_row.is_some() {
                Ok(scoped_row)
            } else {
                sqlx::query(LOAD_RECHARGE_PACK_BY_ID_PUBLIC)
                    .bind(package_id)
                    .bind(&command.requested_at)
                    .fetch_optional(&mut **tx)
                    .await
            }
        }
        .map_err(|error| store_error("failed to load recharge package by id", error))?;
        let Some(row) = row else {
            return Err(CommerceServiceError::conflict(
                "recharge package is unavailable",
            ));
        };
        let pack = map_recharge_pack_row(&row)?;
        ensure_command_matches_package(command, &pack)?;
        return Ok(Some(pack));
    }

    let amount_match = decimal_sql_match_keys(command.amount.as_str());
    let row = if command.tenant_id.trim().is_empty() {
        sqlx::query(LOAD_RECHARGE_PACK_FOR_AMOUNT_PUBLIC)
            .bind(&command.currency_code)
            .bind(command.amount.as_str())
            .bind(&amount_match.compact)
            .bind(&amount_match.one_decimal)
            .bind(&command.requested_at)
            .fetch_optional(&mut **tx)
            .await
    } else {
        let scoped_row = sqlx::query(LOAD_RECHARGE_PACK_FOR_AMOUNT)
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.currency_code)
            .bind(command.amount.as_str())
            .bind(&amount_match.compact)
            .bind(&amount_match.one_decimal)
            .bind(&command.requested_at)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to load recharge package", error))?;
        if scoped_row.is_some() {
            Ok(scoped_row)
        } else {
            sqlx::query(LOAD_RECHARGE_PACK_FOR_AMOUNT_PUBLIC)
                .bind(&command.currency_code)
                .bind(command.amount.as_str())
                .bind(&amount_match.compact)
                .bind(&amount_match.one_decimal)
                .bind(&command.requested_at)
                .fetch_optional(&mut **tx)
                .await
        }
    }
    .map_err(|error| store_error("failed to load recharge package", error))?;

    row.as_ref().map(map_recharge_pack_row).transpose()
}

fn map_recharge_pack_row(
    row: &sqlx::postgres::PgRow,
) -> Result<RechargePack, CommerceServiceError> {
    Ok(RechargePack {
        id: string_cell(row, "package_id"),
        name: string_cell(row, "name"),
        price_amount: commerce_money_cell(row, "price_amount", "recharge package price amount")?,
        currency_code: string_cell(row, "currency_code")
            .trim()
            .to_ascii_uppercase(),
        bonus_points: required_non_negative_integer_cell(row, "bonus_points")?,
        sku_id: string_cell(row, "sku_id"),
    })
}

fn ensure_command_matches_package(
    command: &CreatePointsRechargeOrderCommand,
    pack: &RechargePack,
) -> Result<(), CommerceServiceError> {
    if pack.currency_code != command.currency_code {
        return Err(CommerceServiceError::validation(
            "recharge currency code does not match package currency",
        ));
    }
    if pack.price_amount != command.amount {
        return Err(CommerceServiceError::validation(
            "recharge amount does not match package amount",
        ));
    }
    Ok(())
}

async fn load_recharge_product_sku(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
    pack: Option<&RechargePack>,
) -> Result<RechargeProductSku, CommerceServiceError> {
    if let Some(pack) = pack {
        let row = sqlx::query(LOAD_RECHARGE_PRODUCT_SKU_BY_ID)
            .bind(&pack.sku_id)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to load recharge product sku by id", error))?
            .ok_or_else(|| CommerceServiceError::conflict("recharge product sku is unavailable"))?;

        return Ok(RechargeProductSku {
            sku_id: string_cell(&row, "sku_id"),
            product_name: string_cell(&row, "product_name"),
        });
    }

    let amount_match = decimal_sql_match_keys(command.amount.as_str());
    let row = if command.tenant_id.trim().is_empty() {
        sqlx::query(LOAD_RECHARGE_PRODUCT_SKU_FOR_AMOUNT_PUBLIC)
            .bind(&command.currency_code)
            .bind(command.amount.as_str())
            .bind(&amount_match.compact)
            .bind(&amount_match.one_decimal)
            .fetch_optional(&mut **tx)
            .await
    } else {
        let scoped_row = sqlx::query(LOAD_RECHARGE_PRODUCT_SKU_FOR_AMOUNT)
            .bind(&command.tenant_id)
            .bind(command.organization_id.as_deref())
            .bind(&command.currency_code)
            .bind(command.amount.as_str())
            .bind(&amount_match.compact)
            .bind(&amount_match.one_decimal)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|error| store_error("failed to load recharge product sku", error))?;
        if scoped_row.is_some() {
            Ok(scoped_row)
        } else {
            sqlx::query(LOAD_RECHARGE_PRODUCT_SKU_FOR_AMOUNT_PUBLIC)
                .bind(&command.currency_code)
                .bind(command.amount.as_str())
                .bind(&amount_match.compact)
                .bind(&amount_match.one_decimal)
                .fetch_optional(&mut **tx)
                .await
        }
    }
    .map_err(|error| store_error("failed to load recharge product sku", error))?
    .ok_or_else(|| CommerceServiceError::conflict("recharge product sku is unavailable"))?;

    Ok(RechargeProductSku {
        sku_id: string_cell(&row, "sku_id"),
        product_name: string_cell(&row, "product_name"),
    })
}

async fn load_reusable_recharge_checkout_status(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
    pack: Option<&RechargePack>,
    credited_points: i64,
) -> Result<Option<CheckoutStatusSnapshot>, CommerceServiceError> {
    let amount_match = decimal_sql_match_keys(command.amount.as_str());
    let row = sqlx::query(LOAD_REUSABLE_RECHARGE_CHECKOUT)
        .bind(&command.tenant_id)
        .bind(command.organization_id.as_deref())
        .bind(&command.owner_user_id)
        .bind(command.amount.as_str())
        .bind(&amount_match.compact)
        .bind(&amount_match.one_decimal)
        .bind(&command.currency_code)
        .bind(credited_points)
        .bind(pack.map(|item| item.id.as_str()))
        .bind(&command.requested_at)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| store_error("failed to load reusable recharge checkout", error))?;

    row.as_ref().map(map_checkout_status).transpose()
}

async fn insert_order(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_order
            (id, tenant_id, organization_id, owner_user_id, order_no, status, payment_status, fulfillment_status, refund_status, subject, currency_code, request_no, idempotency_key, created_at, paid_at, cancelled_at, expired_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, 'pending_payment', 'pending', 'unfulfilled', 'none', 'points_recharge', $6, $7, $8, $9, NULL, NULL, $10, $9)
        "#,
    )
    .bind(&command.order_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .bind(&command.order_no)
    .bind(&command.currency_code)
    .bind(&command.order_no)
    .bind(&command.idempotency_key)
    .bind(&command.requested_at)
    .bind(&command.expire_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge order", error))?;
    Ok(())
}

async fn insert_order_item(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
    product: &RechargeProductSku,
    product_name: &str,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_order_item
            (id, tenant_id, order_id, sku_id, sku_snapshot_json, title, quantity, unit_price_amount, total_amount, fulfillment_status, refund_status, created_at)
        VALUES
            ($1, CAST($2 AS TEXT), $3, $4, $5, $6, 1, $7, $7, 'unfulfilled', 'none', $8)
        "#,
    )
    .bind(&command.order_item_id)
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(&product.sku_id)
    .bind(recharge_order_item_snapshot_json(product, product_name))
    .bind(product_name)
    .bind(command.amount.as_str())
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge order item", error))?;
    Ok(())
}

async fn insert_order_amount_breakdown(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_order_amount_breakdown
            (id, tenant_id, order_id, original_amount, discount_amount, payable_amount, currency_code, created_at)
        VALUES
            ($1, CAST($2 AS TEXT), $3, $4, '0.00', $4, $5, $6)
        "#,
    )
    .bind(format!("{}-amount", command.order_id))
    .bind(&command.tenant_id)
    .bind(&command.order_id)
    .bind(command.amount.as_str())
    .bind(&command.currency_code)
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge order amount breakdown", error))?;
    Ok(())
}

async fn insert_payment(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
    method: &RechargeMethod,
    credited_points: i64,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_intent
            (id, tenant_id, organization_id, owner_user_id, order_id, payment_intent_no, payment_method, provider_code, amount, currency_code, status, request_no, idempotency_key, created_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $14)
        "#,
    )
    .bind(&command.payment_intent_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .bind(&command.order_id)
    .bind(format!("PAY-{}", command.order_no))
    .bind(&method.method_key)
    .bind(&method.provider_code)
    .bind(command.amount.as_str())
    .bind(&command.currency_code)
    .bind(CommercePaymentStatus::Pending.as_str())
    .bind(&command.order_no)
    .bind(&command.idempotency_key)
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge payment intent", error))?;
    sqlx::query(
        r#"
        INSERT INTO commerce_payment_attempt
            (id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id, payment_method, provider_code, out_trade_no, amount, currency_code, status, callback_payload, created_at, paid_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, NULL, $14)
        "#,
    )
    .bind(&command.payment_attempt_id)
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .bind(&command.payment_intent_id)
    .bind(&command.order_id)
    .bind(&method.method_key)
    .bind(&method.provider_code)
    .bind(&command.out_trade_no)
    .bind(command.amount.as_str())
    .bind(&command.currency_code)
    .bind(CommercePaymentStatus::Pending.as_str())
    .bind(recharge_payment_callback_payload(
        credited_points,
        command.package_id.as_deref(),
        command.client_request_no.as_deref(),
        command.source.as_deref(),
    ))
    .bind(&command.requested_at)
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge payment attempt", error))?;
    Ok(())
}

async fn insert_recharge_billing_history(
    tx: &mut Transaction<'_, Postgres>,
    command: &CreatePointsRechargeOrderCommand,
    method: &RechargeMethod,
    credited_points: i64,
) -> Result<(), CommerceServiceError> {
    sqlx::query(
        r#"
        INSERT INTO commerce_billing_history
            (id, tenant_id, organization_id, owner_user_id, history_no, history_type,
             direction, asset_type, amount, currency_code, points_delta, status,
             title, reference_no, source_type, source_id, related_order_id,
             related_order_no, payment_method, occurred_at, metadata_json, created_at, updated_at)
        VALUES
            ($1, CAST($2 AS TEXT), CAST($3 AS TEXT), CAST($4 AS TEXT), $5, 'recharge',
             'credit', 'points', $6, $7, $8, 'pending',
             'Recharge', $9, 'commerce_order', $10, $11,
             $12, $13, $14, $15, $14, $14)
        ON CONFLICT (tenant_id, source_type, source_id) DO NOTHING
        "#,
    )
    .bind(format!("billing-history-{}", command.order_id))
    .bind(&command.tenant_id)
    .bind(command.organization_id.as_deref())
    .bind(&command.owner_user_id)
    .bind(format!("BH-{}", command.order_no))
    .bind(command.amount.as_str())
    .bind(&command.currency_code)
    .bind(credited_points)
    .bind(&command.order_no)
    .bind(&command.order_id)
    .bind(&command.order_id)
    .bind(&command.order_no)
    .bind(&method.method_key)
    .bind(&command.requested_at)
    .bind(recharge_history_metadata_json(command))
    .execute(&mut **tx)
    .await
    .map_err(|error| store_error("failed to insert recharge billing history", error))?;
    Ok(())
}

fn recharge_history_metadata_json(command: &CreatePointsRechargeOrderCommand) -> String {
    serde_json::json!({
        "clientRequestNo": command.client_request_no,
        "packageId": command.package_id,
        "source": command.source,
        "currencyCode": command.currency_code,
    })
    .to_string()
}

fn recharge_order_item_snapshot_json(product: &RechargeProductSku, product_name: &str) -> String {
    serde_json::json!({
        "skuId": product.sku_id,
        "productName": product_name,
    })
    .to_string()
}

fn recharge_payment_callback_payload(
    credited_points: i64,
    package_id: Option<&str>,
    client_request_no: Option<&str>,
    source: Option<&str>,
) -> String {
    serde_json::json!({
        "points": credited_points,
        "packageId": package_id,
        "clientRequestNo": client_request_no,
        "source": source,
    })
    .to_string()
}

fn recharge_outcome_from_checkout_status(
    status: CheckoutStatusSnapshot,
) -> CreatePointsRechargeOrderOutcome {
    CreatePointsRechargeOrderOutcome {
        success: true,
        order_no: status.order_no,
        out_trade_no: status.out_trade_no,
        amount: status.amount,
        currency_code: status.currency_code,
        points: status.points,
        provider_code: status.provider_code,
        payment_method: status.payment_method,
        payment_product: status.payment_product,
        status: status.status,
        next_action: status.next_action,
        cashier_url: status.cashier_url,
        qr_code_payload: status.qr_code_payload,
        request_payment_payload: status.request_payment_payload,
    }
}

fn map_checkout_status(
    row: &sqlx::postgres::PgRow,
) -> Result<CheckoutStatusSnapshot, CommerceServiceError> {
    let order_status_value = required_status_cell(row, "order_status", "order")?;
    let order_status = order_status_label(&order_status_value)?.to_owned();
    let payment_status_value = related_status_cell(row, "payment_id", "payment_status", "payment")?;
    let payment_status = payment_status_label(&payment_status_value)?.to_owned();
    let payment_attempt_status_value = related_status_cell(
        row,
        "payment_attempt_id",
        "payment_attempt_status",
        "payment attempt",
    )?;
    let payment_attempt_status = payment_status_label(&payment_attempt_status_value)?.to_owned();
    let recharge_status =
        checkout_effective_recharge_status(&order_status, &payment_status, &payment_attempt_status);
    let status = checkout_status_label(
        &order_status,
        &payment_status,
        &payment_attempt_status,
        &recharge_status,
    );
    let out_trade_no = string_cell(row, "out_trade_no");

    let payment_method = normalize_method_key(&string_cell(row, "payment_method"));

    Ok(CheckoutStatusSnapshot {
        order_no: string_cell(row, "order_no"),
        out_trade_no: out_trade_no.clone(),
        amount: commerce_money_cell(row, "amount", "checkout amount")?,
        currency_code: string_cell(row, "currency_code")
            .trim()
            .to_ascii_uppercase(),
        points: checkout_points(&string_cell(row, "points_value"))?,
        provider_code: string_cell(row, "provider_code"),
        payment_method: payment_method.clone(),
        payment_product: recharge_payment_product(&payment_method)?.to_string(),
        order_status,
        payment_status: checkout_effective_payment_status(&payment_status, &payment_attempt_status),
        recharge_status,
        status: status.to_string(),
        created_at: string_cell(row, "created_at"),
        expires_at: string_cell(row, "expires_at"),
        paid_at: string_cell(row, "paid_at"),
        next_action: checkout_next_action(status).to_string(),
        cashier_url: recharge_cashier_url(&string_cell(row, "order_no"), &out_trade_no),
        qr_code_payload: recharge_cashier_url(&string_cell(row, "order_no"), &out_trade_no),
        request_payment_payload: None,
    })
}

fn checkout_status_label(
    order_status: &str,
    payment_status: &str,
    payment_attempt_status: &str,
    recharge_status: &str,
) -> &'static str {
    if order_status == "refunded" {
        "refunded"
    } else if order_status == "refunding" {
        "refunding"
    } else if recharge_status == "success"
        || payment_attempt_status == "success"
        || payment_status == "success"
        || order_status == "success"
    {
        "success"
    } else if payment_attempt_status == "failed"
        || payment_status == "failed"
        || recharge_status == "failed"
    {
        "failed"
    } else if payment_attempt_status == "expired"
        || payment_status == "expired"
        || order_status == "expired"
    {
        "expired"
    } else {
        "pending"
    }
}

fn checkout_effective_recharge_status(
    order_status: &str,
    payment_status: &str,
    payment_attempt_status: &str,
) -> String {
    if payment_attempt_status == "success"
        || payment_status == "success"
        || order_status == "success"
    {
        "success".to_string()
    } else if payment_attempt_status == "failed"
        || payment_status == "failed"
        || order_status == "failed"
    {
        "failed".to_string()
    } else if payment_attempt_status == "expired"
        || payment_status == "expired"
        || order_status == "expired"
    {
        "expired".to_string()
    } else {
        "pending".to_string()
    }
}

fn checkout_effective_payment_status(payment_status: &str, payment_attempt_status: &str) -> String {
    if payment_attempt_status == "success" {
        "success".to_string()
    } else if payment_attempt_status == "failed" {
        "failed".to_string()
    } else if payment_attempt_status == "expired" {
        "expired".to_string()
    } else if payment_status == "success" {
        "success".to_string()
    } else {
        payment_status.to_string()
    }
}

fn checkout_next_action(status: &str) -> &'static str {
    match status {
        "success" => "completed",
        "failed" | "expired" | "refunding" => "pending",
        "refunded" => "completed",
        _ => "scan_qr",
    }
}

fn recharge_cashier_url(order_no: &str, out_trade_no: &str) -> String {
    format!(
        "https://im.sdkwork.com/cashier?scene=recharge&orderId={order_no}&outTradeNo={out_trade_no}"
    )
}

fn order_status_label(value: &str) -> Result<&'static str, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "draft" | "pending_payment" | "pending" => Ok("pending"),
        "paid" | "fulfilled" | "completed" => Ok("success"),
        "cancelled" | "canceled" => Ok("failed"),
        "expired" => Ok("expired"),
        "refunding" => Ok("refunding"),
        "refunded" => Ok("refunded"),
        status => Err(CommerceServiceError::storage(format!(
            "unsupported checkout order status: {status}"
        ))),
    }
}

fn payment_status_label(value: &str) -> Result<&'static str, CommerceServiceError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" => Ok("pending"),
        status if status == CommercePaymentStatus::Pending.as_str() => Ok("pending"),
        status if status == CommercePaymentStatus::Succeeded.as_str() => Ok("success"),
        status if status == CommercePaymentStatus::Failed.as_str() => Ok("failed"),
        status if status == CommercePaymentStatus::Canceled.as_str() => Ok("expired"),
        status => Err(CommerceServiceError::storage(format!(
            "unsupported checkout payment status: {status}"
        ))),
    }
}

fn checkout_points(value: &str) -> Result<i64, CommerceServiceError> {
    let points = value
        .trim()
        .parse::<i64>()
        .map_err(|_| CommerceServiceError::storage(format!("invalid checkout points: {value}")))?;
    if points < 0 {
        return Err(CommerceServiceError::storage(format!(
            "invalid checkout points: {value}"
        )));
    }
    Ok(points)
}

fn compute_grant_amount(
    amount: &str,
    currency_code: &str,
    bonus_points: i64,
    settings: &RechargeSettingsModel,
) -> Result<i64, CommerceServiceError> {
    let amount_scaled = decimal_to_scaled_i128(amount, 2)?;
    if amount_scaled <= 0 {
        return Err(CommerceServiceError::validation(
            "recharge amount must be greater than zero",
        ));
    }
    let base_points_scaled = decimal_to_scaled_i128(&settings.base_points_per_cny, 6)?;
    let currency_rate = settings
        .currency_to_cny_rates
        .get(&currency_code.trim().to_ascii_uppercase())
        .cloned()
        .unwrap_or_else(|| {
            settings
                .currency_to_cny_rates
                .get(DEFAULT_BASE_CURRENCY_CODE)
                .cloned()
                .unwrap_or_else(|| "1".to_string())
        });
    let currency_rate_scaled = decimal_to_scaled_i128(&currency_rate, 6)?;
    let numerator = amount_scaled
        .checked_mul(currency_rate_scaled)
        .and_then(|value| value.checked_mul(base_points_scaled))
        .ok_or_else(|| CommerceServiceError::storage("recharge credited points overflow"))?;
    let denominator = 100_i128 * 1_000_000_i128 * 1_000_000_i128;
    let rounded = round_divide_i128(numerator, denominator);
    let credited_points = rounded
        .checked_add(i128::from(bonus_points))
        .ok_or_else(|| CommerceServiceError::storage("recharge credited points overflow"))?;
    i64::try_from(credited_points)
        .map_err(|_| CommerceServiceError::storage("recharge credited points overflow"))
}

fn round_divide_i128(numerator: i128, denominator: i128) -> i128 {
    if denominator == 0 {
        return 0;
    }
    if numerator >= 0 {
        (numerator + denominator / 2) / denominator
    } else {
        (numerator - denominator / 2) / denominator
    }
}

fn decimal_to_scaled_i128(value: &str, scale: usize) -> Result<i128, CommerceServiceError> {
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(CommerceServiceError::storage(
            "decimal value must not be empty",
        ));
    }
    let mut parts = normalized.split('.');
    let whole = parts
        .next()
        .unwrap_or_default()
        .parse::<i128>()
        .map_err(|_| CommerceServiceError::storage(format!("invalid decimal value: {value}")))?;
    let fraction = parts.next().unwrap_or_default();
    if parts.next().is_some() || fraction.len() > scale {
        return Err(CommerceServiceError::storage(format!(
            "invalid decimal value: {value}"
        )));
    }
    let mut padded = fraction.to_string();
    while padded.len() < scale {
        padded.push('0');
    }
    let fraction_scaled = if padded.is_empty() {
        0
    } else {
        padded
            .parse::<i128>()
            .map_err(|_| CommerceServiceError::storage(format!("invalid decimal value: {value}")))?
    };
    whole
        .checked_mul(10_i128.pow(scale as u32))
        .and_then(|scaled| scaled.checked_add(fraction_scaled))
        .ok_or_else(|| CommerceServiceError::storage(format!("invalid decimal value: {value}")))
}

fn normalize_decimal_string(value: &str) -> String {
    let trimmed = value.trim();
    if !trimmed.contains('.') {
        return trimmed.to_string();
    }
    let normalized = trimmed.trim_end_matches('0').trim_end_matches('.');
    if normalized.is_empty() {
        "0".to_string()
    } else {
        normalized.to_string()
    }
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

fn format_money_minor(cents: i64) -> String {
    let sign = if cents < 0 { "-" } else { "" };
    let abs = cents.abs();
    format!("{sign}{}.{:02}", abs / 100, abs % 100)
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
        .ok_or_else(|| missing_checkout_status_error(source))
}

fn related_status_cell(
    row: &sqlx::postgres::PgRow,
    relation_column: &str,
    status_column: &str,
    source: &str,
) -> Result<String, CommerceServiceError> {
    if optional_string_cell(row, relation_column)
        .map(|value| value.trim().is_empty())
        .unwrap_or(true)
    {
        return Ok(String::new());
    }
    required_status_cell(row, status_column, source)
}

fn missing_checkout_status_error(source: &str) -> CommerceServiceError {
    match source {
        "order" => CommerceServiceError::storage("missing checkout order status from database row"),
        "payment" => {
            CommerceServiceError::storage("missing checkout payment status from database row")
        }
        value => CommerceServiceError::storage(format!(
            "missing checkout {value} status from database row"
        )),
    }
}

fn required_non_negative_integer_cell(
    row: &sqlx::postgres::PgRow,
    column: &str,
) -> Result<i64, CommerceServiceError> {
    let value = row
        .try_get::<Option<i64>, _>(column)
        .ok()
        .flatten()
        .or_else(|| {
            row.try_get::<Option<i32>, _>(column)
                .ok()
                .flatten()
                .map(i64::from)
        })
        .or_else(|| {
            optional_string_cell(row, column).and_then(|value| value.trim().parse::<i64>().ok())
        })
        .ok_or_else(|| CommerceServiceError::storage(format!("invalid integer column {column}")))?;
    if value < 0 {
        return Err(CommerceServiceError::storage(format!(
            "integer column {column} must not be negative"
        )));
    }
    Ok(value)
}

struct DecimalSqlMatchKeys {
    compact: String,
    one_decimal: String,
}

fn decimal_sql_match_keys(amount: &str) -> DecimalSqlMatchKeys {
    let compact = amount
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string();
    let one_decimal = match amount.split_once('.') {
        Some((whole, fraction)) if fraction.len() == 2 && fraction.ends_with('0') => {
            format!("{}.{}", whole, &fraction[..1])
        }
        _ => amount.to_string(),
    };
    DecimalSqlMatchKeys {
        compact,
        one_decimal,
    }
}

fn normalize_method_key(method: &str) -> String {
    method.trim().to_ascii_lowercase()
}

fn recharge_payment_product(method: &str) -> Result<&'static str, CommerceServiceError> {
    match method.trim().to_ascii_lowercase().as_str() {
        "wechat_pay" => Ok("wechat_native"),
        "alipay" => Ok("alipay_page"),
        "paypal" => Ok("paypal_checkout"),
        "card" => Ok("card"),
        "apple_pay" => Ok("apple_pay"),
        "google_pay" => Ok("google_pay"),
        "wallet_balance" => Ok("wallet_balance"),
        _ => Err(CommerceServiceError::conflict(
            "recharge payment method is unavailable",
        )),
    }
}

fn current_query_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    format_unix_timestamp(seconds)
}

fn format_unix_timestamp(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let seconds_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_of_day / 3_600;
    let minute = (seconds_of_day % 3_600) / 60;
    let second = seconds_of_day % 60;
    format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
}

fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

fn store_error(context: &str, error: sqlx::Error) -> CommerceServiceError {
    CommerceServiceError::storage(format!("{context}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_recharge_points_math_uses_currency_rate_and_bonus() {
        let settings = RechargeSettingsModel {
            base_currency_code: "CNY".to_string(),
            base_points_per_cny: "10".to_string(),
            currency_to_cny_rates: BTreeMap::from([
                ("CNY".to_string(), "1".to_string()),
                ("USD".to_string(), "7.5".to_string()),
            ]),
        };

        assert_eq!(
            compute_grant_amount("12.00", "CNY", 30, &settings).unwrap(),
            150
        );
        assert_eq!(
            compute_grant_amount("20.00", "USD", 50, &settings).unwrap(),
            1550
        );
    }

    #[test]
    fn postgres_recharge_integer_cells_never_parse_through_f64() {
        let source = include_str!("postgres_recharge.rs");
        let forbidden = ["parse", "::<", "f64"].join("");

        assert!(!source.contains(&forbidden));
        assert!(source.contains("required_non_negative_integer_cell"));
    }
}
