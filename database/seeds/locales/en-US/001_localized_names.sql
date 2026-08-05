-- sdkwork:seed-locale en-US
-- Localized en-US display names for the payment method catalog, provider
-- catalog, and channel inventory seeded by database/seeds/common.
-- Each locale file only manages its own keys through jsonb_set, so repeated
-- seeds are idempotent and locales never overwrite each other.

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Credit / Debit Card"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'stripe_card'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Apple Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'stripe_apple_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Google Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'stripe_google_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay (cross-border)"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'stripe_alipay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay (cross-border)"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'stripe_wechat_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay In-store QR"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'alipay_qr'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay PC Website"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'alipay_pc'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay WAP"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'alipay_wap'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay App"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'alipay_app'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay JSAPI"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'alipay_jsapi'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay Native"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'wechat_native'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay JSAPI"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'wechat_jsapi'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay H5"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'wechat_h5'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay App"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'wechat_app'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay Recharge"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'wechat_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_method
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Sandbox Test"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND method_key = 'sandbox_test'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Stripe Card"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-stripe-card'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Stripe Apple Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-stripe-apple-pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Stripe Google Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-stripe-google-pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Stripe Alipay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-stripe-alipay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Stripe WeChat Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-stripe-wechat-pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Alipay QR"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-alipay-qr'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Alipay PC"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-alipay-pc'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Alipay WAP"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-alipay-wap'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Alipay App"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-alipay-app'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Alipay JSAPI"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-alipay-jsapi'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"WeChat Pay Native"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-wechat-native'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"WeChat Pay JSAPI"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-wechat-jsapi'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"WeChat Pay H5"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-wechat-h5'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"WeChat Pay App"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-wechat-app'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"WeChat Pay Recharge"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-recharge-wechat-pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_channel
SET channel_name_i18n = jsonb_set(channel_name_i18n, '{en-US}', '"Sandbox Test"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND channel_no = 'bootstrap-sandbox-test'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Stripe"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'stripe'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Alipay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'alipay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"WeChat Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'wechat_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"PayPal"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'paypal'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Apple Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'apple_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Google Pay"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'google_pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider
SET display_name_i18n = jsonb_set(display_name_i18n, '{en-US}', '"Sandbox"'::jsonb, true)
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_code = 'sandbox'
  AND deleted_at IS NULL;

-- Provider account names. The sandbox bootstrap row is shared by the
-- development and test profiles, so the localized name follows the row's
-- environment.
UPDATE commerce_payment_provider_account
SET account_name_i18n = jsonb_set(account_name_i18n, '{en-US}', '"Stripe Production Account"'::jsonb, true)
WHERE tenant_id = '100001'
  AND id = 'bootstrap-payment-provider-stripe'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider_account
SET account_name_i18n = jsonb_set(account_name_i18n, '{en-US}', '"Alipay Production Account"'::jsonb, true)
WHERE tenant_id = '100001'
  AND id = 'bootstrap-payment-provider-alipay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider_account
SET account_name_i18n = jsonb_set(account_name_i18n, '{en-US}', '"WeChat Pay Production Account"'::jsonb, true)
WHERE tenant_id = '100001'
  AND id = 'bootstrap-payment-provider-wechat-pay'
  AND deleted_at IS NULL;

UPDATE commerce_payment_provider_account
SET account_name_i18n = jsonb_set(
        account_name_i18n,
        '{en-US}',
        CASE environment
            WHEN 'sandbox' THEN '"Sandbox Test Account"'::jsonb
            ELSE '"Sandbox Development Account"'::jsonb
        END,
        true
    )
WHERE tenant_id = '100001'
  AND id = 'bootstrap-payment-provider-sandbox'
  AND deleted_at IS NULL;
