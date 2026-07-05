-- sdkwork:migration
-- id: 0001_payment_store_e2e
-- engine: sqlite
-- module: payment
-- purpose: Minimal commerce order + payment tables for payment repository E2E tests
-- reversible: true
-- transactional: true

CREATE TABLE IF NOT EXISTS commerce_order (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    owner_user_id TEXT NOT NULL,
    order_no TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending_payment',
    subject TEXT NOT NULL,
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    payment_status TEXT,
    fulfillment_status TEXT,
    request_no TEXT,
    idempotency_key TEXT,
    created_at TEXT NOT NULL,
    paid_at TEXT,
    cancelled_at TEXT,
    expired_at TEXT,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commerce_order_item (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    sku_id TEXT,
    sku_snapshot_json TEXT,
    title TEXT,
    quantity INTEGER NOT NULL DEFAULT 1,
    unit_price_amount TEXT,
    total_amount TEXT,
    fulfillment_status TEXT,
    refund_status TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commerce_order_amount_breakdown (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    allocation_type TEXT NOT NULL,
    payable_amount TEXT NOT NULL,
    discount_amount TEXT NOT NULL DEFAULT '0',
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS commerce_payment_intent (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    owner_user_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    payment_intent_no TEXT NOT NULL,
    payment_method TEXT NOT NULL DEFAULT 'wechat_pay',
    provider_code TEXT NOT NULL DEFAULT 'wechat_pay',
    amount TEXT NOT NULL DEFAULT '0',
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL DEFAULT 'pending',
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS commerce_payment_attempt (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    owner_user_id TEXT NOT NULL,
    payment_intent_id TEXT NOT NULL,
    order_id TEXT NOT NULL,
    attempt_no TEXT,
    payment_method TEXT NOT NULL DEFAULT 'wechat_pay',
    provider_code TEXT NOT NULL DEFAULT 'wechat_pay',
    channel_id TEXT,
    out_trade_no TEXT,
    amount TEXT NOT NULL DEFAULT '0',
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL DEFAULT 'pending',
    provider_transaction_id TEXT,
    callback_payload TEXT NOT NULL DEFAULT '{}',
    paid_at TEXT,
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS commerce_refund (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    order_id TEXT NOT NULL,
    payment_attempt_id TEXT NOT NULL,
    refund_no TEXT NOT NULL,
    amount TEXT NOT NULL DEFAULT '0',
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    status TEXT NOT NULL DEFAULT 'submitted',
    refund_reason_code TEXT,
    requested_by_type TEXT NOT NULL DEFAULT 'buyer',
    requested_by TEXT,
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_refund_idempotency
    ON commerce_refund (tenant_id, order_id, idempotency_key)
    WHERE deleted_at IS NULL;

CREATE TABLE IF NOT EXISTS commerce_payment_method (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    method_key TEXT NOT NULL,
    display_name TEXT NOT NULL,
    provider_code TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active',
    sort_order INTEGER NOT NULL DEFAULT 0,
    scope TEXT NOT NULL DEFAULT 'tenant',
    currency_code TEXT NOT NULL DEFAULT 'CNY',
    country_code TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',
    request_no TEXT,
    idempotency_key TEXT NOT NULL,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    deleted_at TEXT
);

CREATE TABLE IF NOT EXISTS commerce_payment_webhook_event (
    id TEXT NOT NULL PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    event_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    provider_code TEXT,
    payload TEXT NOT NULL DEFAULT '{}',
    status TEXT NOT NULL DEFAULT 'queued',
    retries INTEGER NOT NULL DEFAULT 0,
    last_error TEXT,
    received_at TEXT NOT NULL,
    processed_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_webhook_event_event_id
    ON commerce_payment_webhook_event (tenant_id, event_id);
