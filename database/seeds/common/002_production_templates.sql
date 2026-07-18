-- External PSP templates are intentionally inactive until an operator attaches
-- approved secret references and enables the reviewed payment path.
WITH seed (
    id, tenant_id, organization_id, account_no, provider_code, merchant_id,
    environment, settlement_currency, secret_ref, webhook_secret_ref,
    certificate_ref, capabilities, status, metadata
) AS (
    VALUES
        ('bootstrap-payment-provider-stripe', '100001', '0', 'bootstrap-stripe-default', 'stripe', NULL, 'production', 'CNY', 'SDKWORK_PAYMENT_STRIPE_SECRET_KEY', 'SDKWORK_PAYMENT_STRIPE_WEBHOOK_SECRET', NULL, '{"pay":true,"refund":true,"close":true,"query":true}', 'inactive', '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-provider-alipay', '100001', '0', 'bootstrap-alipay-default', 'alipay', NULL, 'production', 'CNY', 'SDKWORK_PAYMENT_ALIPAY_PRIVATE_KEY', 'SDKWORK_PAYMENT_ALIPAY_PUBLIC_KEY', 'SDKWORK_PAYMENT_ALIPAY_PUBLIC_KEY', '{"pay":true,"refund":true,"close":true,"query":true}', 'inactive', '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-provider-wechat-pay', '100001', '0', 'bootstrap-wechat-pay-default', 'wechat_pay', NULL, 'production', 'CNY', 'SDKWORK_PAYMENT_WECHAT_PAY_API_V3_KEY', 'SDKWORK_PAYMENT_WECHAT_PAY_API_V3_KEY', 'SDKWORK_PAYMENT_WECHAT_PAY_CERTIFICATE', '{"pay":true,"refund":true,"close":true,"query":true}', 'inactive', '{"bootstrap":true,"configureBeforeActivation":true}')
)
INSERT INTO commerce_payment_provider_account (
    id, tenant_id, organization_id, account_no, provider_code, merchant_id,
    environment, settlement_currency, secret_ref, webhook_secret_ref,
    certificate_ref, capabilities, status, metadata, created_at, updated_at
)
SELECT
    id, tenant_id, organization_id, account_no, provider_code, merchant_id,
    environment, settlement_currency, secret_ref, webhook_secret_ref,
    certificate_ref, capabilities, status, metadata, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM seed
WHERE NOT EXISTS (
    SELECT 1
    FROM commerce_payment_provider_account existing
    WHERE existing.tenant_id = seed.tenant_id
      AND existing.organization_id = seed.organization_id
      AND existing.account_no = seed.account_no
      AND existing.deleted_at IS NULL
);

WITH seed (
    id, tenant_id, organization_id, channel_no, channel_name, provider_account_id,
    method_id, provider_code, scene_code, currency_code, country_code, status,
    priority, sort_order, metadata
) AS (
    VALUES
        ('bootstrap-payment-channel-stripe-card', '100001', '0', 'bootstrap-stripe-card', 'Stripe Card', 'bootstrap-payment-provider-stripe', 'bootstrap-payment-method-stripe-card', 'stripe', 'web', 'CNY', NULL, 'inactive', 100, 100, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-stripe-apple-pay', '100001', '0', 'bootstrap-stripe-apple-pay', 'Stripe Apple Pay', 'bootstrap-payment-provider-stripe', 'bootstrap-payment-method-stripe-apple-pay', 'stripe', 'web', 'CNY', NULL, 'inactive', 110, 110, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-stripe-google-pay', '100001', '0', 'bootstrap-stripe-google-pay', 'Stripe Google Pay', 'bootstrap-payment-provider-stripe', 'bootstrap-payment-method-stripe-google-pay', 'stripe', 'web', 'CNY', NULL, 'inactive', 120, 120, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-stripe-alipay', '100001', '0', 'bootstrap-stripe-alipay', 'Stripe Alipay', 'bootstrap-payment-provider-stripe', 'bootstrap-payment-method-stripe-alipay', 'stripe', 'web', 'CNY', NULL, 'inactive', 130, 130, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-stripe-wechat-pay', '100001', '0', 'bootstrap-stripe-wechat-pay', 'Stripe WeChat Pay', 'bootstrap-payment-provider-stripe', 'bootstrap-payment-method-stripe-wechat-pay', 'stripe', 'web', 'CNY', NULL, 'inactive', 140, 140, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-alipay-qr', '100001', '0', 'bootstrap-alipay-qr', 'Alipay QR', 'bootstrap-payment-provider-alipay', 'bootstrap-payment-method-alipay-qr', 'alipay', 'api', 'CNY', 'CN', 'inactive', 200, 200, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-alipay-pc', '100001', '0', 'bootstrap-alipay-pc', 'Alipay PC', 'bootstrap-payment-provider-alipay', 'bootstrap-payment-method-alipay-pc', 'alipay', 'web', 'CNY', 'CN', 'inactive', 210, 210, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-alipay-wap', '100001', '0', 'bootstrap-alipay-wap', 'Alipay WAP', 'bootstrap-payment-provider-alipay', 'bootstrap-payment-method-alipay-wap', 'alipay', 'web', 'CNY', 'CN', 'inactive', 220, 220, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-alipay-app', '100001', '0', 'bootstrap-alipay-app', 'Alipay App', 'bootstrap-payment-provider-alipay', 'bootstrap-payment-method-alipay-app', 'alipay', 'app', 'CNY', 'CN', 'inactive', 230, 230, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-alipay-jsapi', '100001', '0', 'bootstrap-alipay-jsapi', 'Alipay JSAPI', 'bootstrap-payment-provider-alipay', 'bootstrap-payment-method-alipay-jsapi', 'alipay', 'mini_program', 'CNY', 'CN', 'inactive', 240, 240, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-wechat-native', '100001', '0', 'bootstrap-wechat-native', 'WeChat Pay Native', 'bootstrap-payment-provider-wechat-pay', 'bootstrap-payment-method-wechat-native', 'wechat_pay', 'api', 'CNY', 'CN', 'inactive', 300, 300, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-wechat-jsapi', '100001', '0', 'bootstrap-wechat-jsapi', 'WeChat Pay JSAPI', 'bootstrap-payment-provider-wechat-pay', 'bootstrap-payment-method-wechat-jsapi', 'wechat_pay', 'mini_program', 'CNY', 'CN', 'inactive', 310, 310, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-wechat-h5', '100001', '0', 'bootstrap-wechat-h5', 'WeChat Pay H5', 'bootstrap-payment-provider-wechat-pay', 'bootstrap-payment-method-wechat-h5', 'wechat_pay', 'web', 'CNY', 'CN', 'inactive', 320, 320, '{"bootstrap":true,"configureBeforeActivation":true}'),
        ('bootstrap-payment-channel-wechat-app', '100001', '0', 'bootstrap-wechat-app', 'WeChat Pay App', 'bootstrap-payment-provider-wechat-pay', 'bootstrap-payment-method-wechat-app', 'wechat_pay', 'app', 'CNY', 'CN', 'inactive', 330, 330, '{"bootstrap":true,"configureBeforeActivation":true}')
)
INSERT INTO commerce_payment_channel (
    id, tenant_id, organization_id, channel_no, channel_name, provider_account_id,
    method_id, provider_code, scene_code, currency_code, country_code, status,
    priority, sort_order, metadata, created_at, updated_at
)
SELECT
    id, tenant_id, organization_id, channel_no, channel_name, provider_account_id,
    method_id, provider_code, scene_code, currency_code, country_code, status,
    priority, sort_order, metadata, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM seed
WHERE NOT EXISTS (
    SELECT 1 FROM commerce_payment_channel existing
    WHERE existing.tenant_id = seed.tenant_id
      AND existing.organization_id = seed.organization_id
      AND existing.channel_no = seed.channel_no
      AND existing.deleted_at IS NULL
);
