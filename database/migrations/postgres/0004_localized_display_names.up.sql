-- Localized display-name maps for payment reference data.
--
-- Per DATABASE_SPEC.md §6.4.1 localized reference data is modeled as stable
-- base rows plus locale-specific translations; the locale map is a single
-- JSONB column per table, not per-language columns. Locale seed files under
-- database/seeds/locales/{locale}/ fill the maps idempotently.
ALTER TABLE commerce_payment_method ADD COLUMN IF NOT EXISTS display_name_i18n JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE commerce_payment_channel ADD COLUMN IF NOT EXISTS channel_name_i18n JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE commerce_payment_provider ADD COLUMN IF NOT EXISTS display_name_i18n JSONB NOT NULL DEFAULT '{}'::jsonb;
