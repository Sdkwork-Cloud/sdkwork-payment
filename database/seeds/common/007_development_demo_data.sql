-- Development-only operator data for the complete Payment admin workflow.
-- This file is selected only by the development seed profile. It contains no
-- usable provider credentials, private keys, or production merchant data.

INSERT INTO commerce_payment_provider_account (
    id, tenant_id, organization_id, account_no, provider_code, merchant_id,
    account_mode, environment, country_code, settlement_currency, secret_ref,
    capabilities, status, metadata, last_tested_at, last_test_status,
    created_at, updated_at
)
VALUES (
    'bootstrap-payment-provider-sandbox-partner', '100001', '100002',
    'bootstrap-sandbox-partner', 'sandbox', 'demo-partner-merchant', 'partner',
    'development', 'CN', 'CNY', 'bootstrap:development-placeholder',
    '{"pay":true,"refund":true,"close":true,"query":true}', 'inactive',
    '{"bootstrap":true,"developmentDemo":true,"credentialState":"not_configured"}',
    '2026-07-20T01:00:00Z', 'unknown', CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
)
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_sub_merchant (
    id, tenant_id, organization_id, provider_account_id,
    external_sub_merchant_id, sub_appid, sub_mch_id, display_name, legal_name,
    status, capabilities, metadata, created_at, updated_at
)
VALUES
    (
        'bootstrap-payment-sub-merchant-demo-store', '100001', '100002',
        'bootstrap-payment-provider-sandbox-partner', 'demo-sub-merchant-001',
        'demo-sub-app-001', 'demo-sub-mch-001', 'Demo Flagship Store',
        'SDKWork Demo Commerce Co., Ltd.', 'active',
        '{"pay":true,"refund":true}',
        '{"bootstrap":true,"developmentDemo":true,"onboardingState":"verified"}',
        '2026-07-18T02:00:00Z', '2026-07-19T08:30:00Z'
    ),
    (
        'bootstrap-payment-sub-merchant-demo-review', '100001', '100002',
        'bootstrap-payment-provider-sandbox-partner', 'demo-sub-merchant-002',
        'demo-sub-app-002', 'demo-sub-mch-002', 'Demo New Store',
        'SDKWork Demo Retail Co., Ltd.', 'pending_review',
        '{"pay":false,"refund":false}',
        '{"bootstrap":true,"developmentDemo":true,"onboardingState":"documents_review"}',
        '2026-07-20T02:20:00Z', '2026-07-20T02:20:00Z'
    )
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_route_rule (
    id, tenant_id, organization_id, rule_no, priority, purchase_type,
    country_code, currency_code, client_platform, amount_min, amount_max,
    user_segment, risk_level, channel_id, status, starts_at, ends_at,
    created_at, updated_at
)
VALUES
    (
        'bootstrap-payment-route-rule-sandbox-default', '100001', '100002',
        'DEV-SANDBOX-DEFAULT', 100, 'goods', 'CN', 'CNY', 'web', 0, 5000,
        'all', 'low', 'bootstrap-payment-channel-sandbox-test', 'active',
        '2026-01-01T00:00:00Z', '2027-01-01T00:00:00Z',
        '2026-07-18T01:00:00Z', '2026-07-20T01:00:00Z'
    ),
    (
        'bootstrap-payment-route-rule-sandbox-review', '100001', '100002',
        'DEV-SANDBOX-REVIEW', 200, 'service', 'CN', 'CNY', 'web', 5000, 50000,
        'business', 'medium', 'bootstrap-payment-channel-sandbox-test', 'inactive',
        '2026-01-01T00:00:00Z', '2027-01-01T00:00:00Z',
        '2026-07-19T01:00:00Z', '2026-07-20T01:00:00Z'
    )
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_intent (
    id, tenant_id, organization_id, owner_user_id, order_id,
    payment_intent_no, payment_method, provider_code, amount, currency_code,
    status, request_no, idempotency_key, created_at, updated_at
)
VALUES
    ('bootstrap-payment-intent-demo-succeeded', '100001', '100002', '1', 'demo-order-20260720-001', 'PI-DEMO-20260720-001', 'sandbox_test', 'sandbox', 299.00, 'CNY', 'succeeded', 'REQ-DEMO-PAY-001', 'demo-payment-intent-001', '2026-07-20T01:20:00Z', '2026-07-20T01:21:08Z'),
    ('bootstrap-payment-intent-demo-processing', '100001', '100002', '1', 'demo-order-20260720-002', 'PI-DEMO-20260720-002', 'sandbox_test', 'sandbox', 88.50, 'CNY', 'processing', 'REQ-DEMO-PAY-002', 'demo-payment-intent-002', '2026-07-20T02:05:00Z', '2026-07-20T02:05:18Z'),
    ('bootstrap-payment-intent-demo-pending', '100001', '100002', '1', 'demo-order-20260720-003', 'PI-DEMO-20260720-003', 'sandbox_test', 'sandbox', 1299.00, 'CNY', 'pending', 'REQ-DEMO-PAY-003', 'demo-payment-intent-003', '2026-07-20T02:40:00Z', '2026-07-20T02:40:00Z'),
    ('bootstrap-payment-intent-demo-failed', '100001', '100002', '1', 'demo-order-20260719-004', 'PI-DEMO-20260719-004', 'sandbox_test', 'sandbox', 52.00, 'CNY', 'failed', 'REQ-DEMO-PAY-004', 'demo-payment-intent-004', '2026-07-19T09:30:00Z', '2026-07-19T09:30:22Z'),
    ('bootstrap-payment-intent-demo-closed', '100001', '100002', '1', 'demo-order-20260718-005', 'PI-DEMO-20260718-005', 'sandbox_test', 'sandbox', 36.80, 'CNY', 'closed', 'REQ-DEMO-PAY-005', 'demo-payment-intent-005', '2026-07-18T06:10:00Z', '2026-07-18T06:25:00Z')
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_attempt (
    id, tenant_id, organization_id, owner_user_id, payment_intent_id, order_id,
    attempt_no, payment_method, provider_code, channel_id, out_trade_no,
    amount, currency_code, status, provider_transaction_id, callback_payload,
    paid_at, request_no, idempotency_key, created_at, updated_at
)
VALUES
    ('bootstrap-payment-attempt-demo-succeeded', '100001', '100002', '1', 'bootstrap-payment-intent-demo-succeeded', 'demo-order-20260720-001', 'PA-DEMO-20260720-001', 'sandbox_test', 'sandbox', 'bootstrap-payment-channel-sandbox-test', 'OUT-DEMO-20260720-001', 299.00, 'CNY', 'succeeded', 'sandbox_txn_demo_001', '{"developmentDemo":true,"result":"approved"}', '2026-07-20T01:21:08Z', 'REQ-DEMO-ATTEMPT-001', 'demo-payment-attempt-001', '2026-07-20T01:20:02Z', '2026-07-20T01:21:08Z'),
    ('bootstrap-payment-attempt-demo-processing', '100001', '100002', '1', 'bootstrap-payment-intent-demo-processing', 'demo-order-20260720-002', 'PA-DEMO-20260720-002', 'sandbox_test', 'sandbox', 'bootstrap-payment-channel-sandbox-test', 'OUT-DEMO-20260720-002', 88.50, 'CNY', 'processing', NULL, '{"developmentDemo":true,"result":"awaiting_callback"}', NULL, 'REQ-DEMO-ATTEMPT-002', 'demo-payment-attempt-002', '2026-07-20T02:05:02Z', '2026-07-20T02:05:18Z'),
    ('bootstrap-payment-attempt-demo-pending', '100001', '100002', '1', 'bootstrap-payment-intent-demo-pending', 'demo-order-20260720-003', 'PA-DEMO-20260720-003', 'sandbox_test', 'sandbox', 'bootstrap-payment-channel-sandbox-test', 'OUT-DEMO-20260720-003', 1299.00, 'CNY', 'pending', NULL, '{"developmentDemo":true,"result":"customer_action_required"}', NULL, 'REQ-DEMO-ATTEMPT-003', 'demo-payment-attempt-003', '2026-07-20T02:40:01Z', '2026-07-20T02:40:01Z'),
    ('bootstrap-payment-attempt-demo-failed', '100001', '100002', '1', 'bootstrap-payment-intent-demo-failed', 'demo-order-20260719-004', 'PA-DEMO-20260719-004', 'sandbox_test', 'sandbox', 'bootstrap-payment-channel-sandbox-test', 'OUT-DEMO-20260719-004', 52.00, 'CNY', 'failed', NULL, '{"developmentDemo":true,"errorCode":"sandbox_declined"}', NULL, 'REQ-DEMO-ATTEMPT-004', 'demo-payment-attempt-004', '2026-07-19T09:30:02Z', '2026-07-19T09:30:22Z'),
    ('bootstrap-payment-attempt-demo-closed', '100001', '100002', '1', 'bootstrap-payment-intent-demo-closed', 'demo-order-20260718-005', 'PA-DEMO-20260718-005', 'sandbox_test', 'sandbox', 'bootstrap-payment-channel-sandbox-test', 'OUT-DEMO-20260718-005', 36.80, 'CNY', 'closed', NULL, '{"developmentDemo":true,"result":"closed_by_operator"}', NULL, 'REQ-DEMO-ATTEMPT-005', 'demo-payment-attempt-005', '2026-07-18T06:10:02Z', '2026-07-18T06:25:00Z')
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_webhook_event (
    id, tenant_id, organization_id, event_id, event_type, provider_code,
    payload, status, retries, last_error, received_at, processed_at,
    created_at, updated_at
)
VALUES
    ('bootstrap-payment-webhook-demo-processed', '100001', '100002', 'evt_demo_payment_succeeded_001', 'payment.succeeded', 'sandbox', '{"developmentDemo":true,"outTradeNo":"OUT-DEMO-20260720-001","signatureStatus":"valid"}', 'processed', 0, NULL, '2026-07-20T01:21:07Z', '2026-07-20T01:21:08Z', '2026-07-20T01:21:07Z', '2026-07-20T01:21:08Z'),
    ('bootstrap-payment-webhook-demo-failed', '100001', '100002', 'evt_demo_payment_failed_001', 'payment.failed', 'sandbox', '{"developmentDemo":true,"outTradeNo":"OUT-DEMO-20260719-004","signatureStatus":"valid"}', 'failed', 2, 'Demo downstream timeout', '2026-07-19T09:30:20Z', NULL, '2026-07-19T09:30:20Z', '2026-07-19T09:32:20Z'),
    ('bootstrap-payment-webhook-demo-queued', '100001', '100002', 'evt_demo_payment_processing_001', 'payment.processing', 'sandbox', '{"developmentDemo":true,"outTradeNo":"OUT-DEMO-20260720-002","signatureStatus":"unverified"}', 'queued', 0, NULL, '2026-07-20T02:05:17Z', NULL, '2026-07-20T02:05:17Z', '2026-07-20T02:05:17Z')
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_reconciliation_run (
    id, tenant_id, organization_id, run_no, provider_code,
    provider_account_id, reconciliation_type, period_start, period_end,
    status, matched_count, mismatched_count, unmatched_count,
    total_difference_amount, currency_code, request_no, idempotency_key,
    created_at, updated_at
)
VALUES
    ('bootstrap-payment-reconciliation-demo-succeeded', '100001', '100002', 'RECON-DEMO-20260719', 'sandbox', 'bootstrap-payment-provider-sandbox', 'daily', '2026-07-19T00:00:00Z', '2026-07-20T00:00:00Z', 'succeeded', 128, 2, 1, 8.50, 'CNY', 'REQ-DEMO-RECON-001', 'demo-reconciliation-001', '2026-07-20T00:10:00Z', '2026-07-20T00:12:30Z'),
    ('bootstrap-payment-reconciliation-demo-running', '100001', '100002', 'RECON-DEMO-20260720', 'sandbox', 'bootstrap-payment-provider-sandbox', 'daily', '2026-07-20T00:00:00Z', '2026-07-21T00:00:00Z', 'running', 37, 0, 0, 0, 'CNY', 'REQ-DEMO-RECON-002', 'demo-reconciliation-002', '2026-07-20T03:00:00Z', '2026-07-20T03:01:00Z'),
    ('bootstrap-payment-reconciliation-demo-failed', '100001', '100002', 'RECON-DEMO-MANUAL-001', 'sandbox', 'bootstrap-payment-provider-sandbox', 'manual', '2026-07-18T00:00:00Z', '2026-07-19T00:00:00Z', 'failed', 0, 0, 0, 0, 'CNY', 'REQ-DEMO-RECON-003', 'demo-reconciliation-003', '2026-07-19T00:10:00Z', '2026-07-19T00:10:20Z')
ON CONFLICT DO NOTHING;

INSERT INTO commerce_payment_certificate (
    id, tenant_id, organization_id, certificate_no, provider_code, kind,
    subject_cn, issuer_cn, serial_number, fingerprint_sha256, content_ref,
    valid_from, valid_until, status, metadata, created_at, updated_at
)
VALUES (
    'bootstrap-payment-certificate-demo-platform', '100001', '100002',
    'CERT-DEMO-SANDBOX-001', 'wechat_pay', 'platform',
    'SDKWork Sandbox Payment Platform', 'SDKWork Development CA',
    'DEMO-SERIAL-20260720',
    'd4b40f2043fe32b7811a63c8f5b09fd302fb7c13d654df1df461049e0c23bb41',
    'bootstrap:development-metadata-only', '2026-07-01T00:00:00Z',
    '2026-10-01T00:00:00Z', 'pending',
    '{"bootstrap":true,"developmentDemo":true,"contentState":"not_configured"}',
    '2026-07-20T01:00:00Z', '2026-07-20T01:00:00Z'
)
ON CONFLICT DO NOTHING;
