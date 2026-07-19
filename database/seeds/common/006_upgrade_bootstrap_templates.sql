-- One-time-compatible repair for databases initialized with the legacy PSP
-- templates. Only untouched bootstrap rows are changed; administrator-owned
-- provider accounts no longer carry the mock/configure marker and are skipped.
--
-- Organization id 0 is an IAM login sentinel and cannot back an organization
-- session. Payment backend-admin routes require an organization session, so
-- move untouched platform bootstrap templates into the stable bootstrap admin
-- organization used by Manager development and initial production setup.
UPDATE commerce_payment_method
SET organization_id = '100002', updated_at = CURRENT_TIMESTAMP
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND id LIKE 'bootstrap-payment-method-%'
  AND CAST(metadata AS TEXT) LIKE '%bootstrap%';

UPDATE commerce_payment_provider_account
SET organization_id = '100002', updated_at = CURRENT_TIMESTAMP
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND id LIKE 'bootstrap-payment-provider-%'
  AND CAST(metadata AS TEXT) LIKE '%bootstrap%';

UPDATE commerce_payment_provider_credential
SET organization_id = '100002', updated_at = CURRENT_TIMESTAMP
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND provider_account_id LIKE 'bootstrap-payment-provider-%';

UPDATE commerce_payment_channel
SET organization_id = '100002', updated_at = CURRENT_TIMESTAMP
WHERE tenant_id = '100001'
  AND organization_id = '0'
  AND id LIKE 'bootstrap-payment-channel-%'
  AND CAST(metadata AS TEXT) LIKE '%bootstrap%';

UPDATE commerce_payment_provider_account
SET merchant_id = 'mock-stripe-account',
    metadata = '{"bootstrap":true,"configurationState":"mock","configureBeforeActivation":true}',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-stripe'
  AND status = 'inactive'
  AND (merchant_id IS NULL OR TRIM(merchant_id) = '')
  AND CAST(metadata AS TEXT) LIKE '%configureBeforeActivation%';

UPDATE commerce_payment_provider_account
SET merchant_id = 'mock-alipay-app-id',
    metadata = '{"bootstrap":true,"configurationState":"mock","appId":"mock-alipay-app-id","configureBeforeActivation":true}',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-alipay'
  AND status = 'inactive'
  AND (merchant_id IS NULL OR TRIM(merchant_id) = '')
  AND CAST(metadata AS TEXT) LIKE '%configureBeforeActivation%';

UPDATE commerce_payment_provider_account
SET merchant_id = 'mock-wechat-mch-id',
    secret_ref = 'database:primary_secret',
    webhook_secret_ref = 'database:webhook_secret',
    certificate_ref = 'database:certificate',
    metadata = '{"bootstrap":true,"configurationState":"mock","appId":"mock-wechat-app-id","merchantSerialNo":"mock-wechat-merchant-serial-no","notifyUrl":"https://mock-payment.example.com/app/v3/api/orders/payments/webhooks/wechat_pay","configureBeforeActivation":true}',
    updated_at = CURRENT_TIMESTAMP
WHERE id = 'bootstrap-payment-provider-wechat-pay'
  AND status = 'inactive'
  AND (
        merchant_id IS NULL
        OR TRIM(merchant_id) = ''
        OR secret_ref = 'SDKWORK_PAYMENT_WECHAT_PAY_API_V3_KEY'
        OR certificate_ref = 'SDKWORK_PAYMENT_WECHAT_PAY_CERTIFICATE'
      )
  AND CAST(metadata AS TEXT) LIKE '%configureBeforeActivation%';

-- Catalog and channels are pre-enabled, while the inactive provider account is
-- the fail-closed routing gate. Once real credentials replace the mock values
-- and the account is activated, no second method/channel activation pass is
-- required.
UPDATE commerce_payment_method
SET status = 'active', updated_at = CURRENT_TIMESTAMP
WHERE id IN (
    'bootstrap-payment-method-stripe-card',
    'bootstrap-payment-method-stripe-apple-pay',
    'bootstrap-payment-method-stripe-google-pay',
    'bootstrap-payment-method-stripe-alipay',
    'bootstrap-payment-method-stripe-wechat-pay',
    'bootstrap-payment-method-alipay-qr',
    'bootstrap-payment-method-alipay-pc',
    'bootstrap-payment-method-alipay-wap',
    'bootstrap-payment-method-alipay-app',
    'bootstrap-payment-method-alipay-jsapi',
    'bootstrap-payment-method-wechat-native',
    'bootstrap-payment-method-wechat-jsapi',
    'bootstrap-payment-method-wechat-h5',
    'bootstrap-payment-method-wechat-app'
)
AND EXISTS (
    SELECT 1
    FROM commerce_payment_provider_account a
    WHERE a.tenant_id = commerce_payment_method.tenant_id
      AND a.provider_code = commerce_payment_method.provider_code
      AND a.status = 'inactive'
      AND a.deleted_at IS NULL
      AND CAST(a.metadata AS TEXT) LIKE '%configurationState%'
);

UPDATE commerce_payment_channel
SET status = 'active', updated_at = CURRENT_TIMESTAMP
WHERE id IN (
    'bootstrap-payment-channel-stripe-card',
    'bootstrap-payment-channel-stripe-apple-pay',
    'bootstrap-payment-channel-stripe-google-pay',
    'bootstrap-payment-channel-stripe-alipay',
    'bootstrap-payment-channel-stripe-wechat-pay',
    'bootstrap-payment-channel-alipay-qr',
    'bootstrap-payment-channel-alipay-pc',
    'bootstrap-payment-channel-alipay-wap',
    'bootstrap-payment-channel-alipay-app',
    'bootstrap-payment-channel-alipay-jsapi',
    'bootstrap-payment-channel-wechat-native',
    'bootstrap-payment-channel-wechat-jsapi',
    'bootstrap-payment-channel-wechat-h5',
    'bootstrap-payment-channel-wechat-app'
)
AND EXISTS (
    SELECT 1
    FROM commerce_payment_provider_account a
    WHERE a.id = commerce_payment_channel.provider_account_id
      AND a.tenant_id = commerce_payment_channel.tenant_id
      AND a.status = 'inactive'
      AND a.deleted_at IS NULL
      AND CAST(a.metadata AS TEXT) LIKE '%configurationState%'
);
