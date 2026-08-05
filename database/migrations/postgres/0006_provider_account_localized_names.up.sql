-- Localized account-name support for payment provider accounts.
--
-- Per DATABASE_SPEC.md §6.4.1 localized reference data is modeled as stable
-- base rows plus locale-specific translations; the locale map is a single
-- JSONB column per table, not per-language columns. account_name carries the
-- canonical operator-facing name (seeded or set by operators), while
-- account_name_i18n holds locale display maps filled idempotently by locale
-- seed files under database/seeds/locales/{locale}/.
ALTER TABLE commerce_payment_provider_account ADD COLUMN IF NOT EXISTS account_name TEXT;
ALTER TABLE commerce_payment_provider_account ADD COLUMN IF NOT EXISTS account_name_i18n JSONB NOT NULL DEFAULT '{}'::jsonb;

-- Back-fill canonical names for bootstrap-seeded accounts created before this
-- migration. Operator-owned accounts are never renamed here; they can set
-- account_name through the admin update flow. The sandbox bootstrap row is
-- shared by the development and test profiles, so the name follows the row's
-- environment.
UPDATE commerce_payment_provider_account
SET account_name = 'Stripe Production Account',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-stripe'
  AND account_name IS NULL;

UPDATE commerce_payment_provider_account
SET account_name = 'Alipay Production Account',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-alipay'
  AND account_name IS NULL;

UPDATE commerce_payment_provider_account
SET account_name = 'WeChat Pay Production Account',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-wechat-pay'
  AND account_name IS NULL;

UPDATE commerce_payment_provider_account
SET account_name = CASE environment
        WHEN 'sandbox' THEN 'Sandbox Test Account'
        ELSE 'Sandbox Development Account'
    END,
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-sandbox'
  AND account_name IS NULL;

UPDATE commerce_payment_provider_account
SET account_name = 'Sandbox Partner Demo Account',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-sandbox-partner'
  AND account_name IS NULL;
