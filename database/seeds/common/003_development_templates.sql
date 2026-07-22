-- Development bootstrap is immediately usable through the local sandbox. Real
-- PSP methods remain present in the catalog but cannot be selected until an
-- operator configures their own account and channel.
INSERT INTO commerce_payment_method (
    id, tenant_id, organization_id, method_key, display_name, provider_code,
    status, sort_order, scope, currency_code, country_code, metadata,
    idempotency_key, created_at, updated_at
)
VALUES
    ('bootstrap-payment-method-sandbox-test', '100001', '0', 'sandbox_test', 'Sandbox Test', 'sandbox', 'active', 900, 'organization', 'CNY', NULL, '{"bootstrap":true,"environment":"development"}', 'bootstrap-payment-method-sandbox-test', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_provider_account (
    id, tenant_id, organization_id, account_no, provider_code, merchant_id,
    environment, settlement_currency, secret_ref, webhook_secret_ref,
    certificate_ref, capabilities, status, metadata, created_at, updated_at
)
VALUES
    ('bootstrap-payment-provider-sandbox', '100001', '0', 'bootstrap-sandbox-default', 'sandbox', NULL, 'development', 'CNY', 'database:primary_secret', NULL, NULL, '{"pay":true,"refund":true,"close":true,"query":true}', 'active', '{"bootstrap":true,"environment":"development"}', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_channel (
    id, tenant_id, organization_id, channel_no, channel_name, provider_account_id,
    method_id, provider_code, scene_code, currency_code, country_code, status,
    priority, sort_order, metadata, created_at, updated_at
)
VALUES
    ('bootstrap-payment-channel-sandbox-test', '100001', '0', 'bootstrap-sandbox-test', 'Sandbox Test', 'bootstrap-payment-provider-sandbox', 'bootstrap-payment-method-sandbox-test', 'sandbox', 'api', 'CNY', NULL, 'active', 900, 900, '{"bootstrap":true,"environment":"development"}', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
ON CONFLICT DO NOTHING;
