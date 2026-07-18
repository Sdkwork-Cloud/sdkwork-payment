-- Shared payment-method catalog for the platform bootstrap tenant. Profile
-- templates add the sandbox method and provider/channel records appropriate to
-- their environment. Existing administrator-owned records are never updated.
WITH seed (
    id, tenant_id, organization_id, method_key, display_name, provider_code,
    status, sort_order, scope, currency_code, country_code, metadata, idempotency_key
) AS (
    VALUES
        ('bootstrap-payment-method-stripe-card', '100001', '0', 'stripe_card', 'Credit / Debit Card', 'stripe', 'inactive', 100, 'organization', 'CNY', NULL, '{"bootstrap":true,"provider":"stripe"}', 'bootstrap-payment-method-stripe-card'),
        ('bootstrap-payment-method-stripe-apple-pay', '100001', '0', 'stripe_apple_pay', 'Apple Pay', 'stripe', 'inactive', 110, 'organization', 'CNY', NULL, '{"bootstrap":true,"provider":"stripe"}', 'bootstrap-payment-method-stripe-apple-pay'),
        ('bootstrap-payment-method-stripe-google-pay', '100001', '0', 'stripe_google_pay', 'Google Pay', 'stripe', 'inactive', 120, 'organization', 'CNY', NULL, '{"bootstrap":true,"provider":"stripe"}', 'bootstrap-payment-method-stripe-google-pay'),
        ('bootstrap-payment-method-stripe-alipay', '100001', '0', 'stripe_alipay', 'Alipay (cross-border)', 'stripe', 'inactive', 130, 'organization', 'CNY', NULL, '{"bootstrap":true,"provider":"stripe"}', 'bootstrap-payment-method-stripe-alipay'),
        ('bootstrap-payment-method-stripe-wechat-pay', '100001', '0', 'stripe_wechat_pay', 'WeChat Pay (cross-border)', 'stripe', 'inactive', 140, 'organization', 'CNY', NULL, '{"bootstrap":true,"provider":"stripe"}', 'bootstrap-payment-method-stripe-wechat-pay'),
        ('bootstrap-payment-method-alipay-qr', '100001', '0', 'alipay_qr', 'Alipay In-store QR', 'alipay', 'inactive', 200, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"alipay"}', 'bootstrap-payment-method-alipay-qr'),
        ('bootstrap-payment-method-alipay-pc', '100001', '0', 'alipay_pc', 'Alipay PC Website', 'alipay', 'inactive', 210, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"alipay"}', 'bootstrap-payment-method-alipay-pc'),
        ('bootstrap-payment-method-alipay-wap', '100001', '0', 'alipay_wap', 'Alipay WAP', 'alipay', 'inactive', 220, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"alipay"}', 'bootstrap-payment-method-alipay-wap'),
        ('bootstrap-payment-method-alipay-app', '100001', '0', 'alipay_app', 'Alipay App', 'alipay', 'inactive', 230, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"alipay"}', 'bootstrap-payment-method-alipay-app'),
        ('bootstrap-payment-method-alipay-jsapi', '100001', '0', 'alipay_jsapi', 'Alipay JSAPI', 'alipay', 'inactive', 240, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"alipay"}', 'bootstrap-payment-method-alipay-jsapi'),
        ('bootstrap-payment-method-wechat-native', '100001', '0', 'wechat_native', 'WeChat Pay Native', 'wechat_pay', 'inactive', 300, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"wechat_pay"}', 'bootstrap-payment-method-wechat-native'),
        ('bootstrap-payment-method-wechat-jsapi', '100001', '0', 'wechat_jsapi', 'WeChat Pay JSAPI', 'wechat_pay', 'inactive', 310, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"wechat_pay"}', 'bootstrap-payment-method-wechat-jsapi'),
        ('bootstrap-payment-method-wechat-h5', '100001', '0', 'wechat_h5', 'WeChat Pay H5', 'wechat_pay', 'inactive', 320, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"wechat_pay"}', 'bootstrap-payment-method-wechat-h5'),
        ('bootstrap-payment-method-wechat-app', '100001', '0', 'wechat_app', 'WeChat Pay App', 'wechat_pay', 'inactive', 330, 'organization', 'CNY', 'CN', '{"bootstrap":true,"provider":"wechat_pay"}', 'bootstrap-payment-method-wechat-app')
)
INSERT INTO commerce_payment_method (
    id, tenant_id, organization_id, method_key, display_name, provider_code,
    status, sort_order, scope, currency_code, country_code, metadata,
    idempotency_key, created_at, updated_at
)
SELECT
    id, tenant_id, organization_id, method_key, display_name, provider_code,
    status, sort_order, scope, currency_code, country_code, metadata,
    idempotency_key, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM seed
WHERE NOT EXISTS (
    SELECT 1
    FROM commerce_payment_method existing
    WHERE existing.tenant_id = seed.tenant_id
      AND existing.organization_id = seed.organization_id
      AND existing.method_key = seed.method_key
      AND existing.deleted_at IS NULL
);
