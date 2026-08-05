-- Canonical webhook delivery table for provider callback ingestion.
--
-- The Cloud Router transit store (payment_callback_store) records each raw
-- provider delivery here before normalizing it into
-- commerce_payment_webhook_event. This table was declared by host table
-- inventories but never owned by a module baseline; this migration makes the
-- canonical schema complete so fresh installs can persist deliveries.
CREATE TABLE IF NOT EXISTS commerce_payment_webhook_delivery (
    id                   TEXT PRIMARY KEY,
    tenant_id            TEXT NOT NULL,
    organization_id      TEXT,
    delivery_no          TEXT NOT NULL,
    provider_code        TEXT NOT NULL,
    provider_account_id  TEXT,
    event_id             TEXT NOT NULL,
    nonce                TEXT,
    request_timestamp    TEXT,
    signature            TEXT,
    signature_algorithm  TEXT,
    headers_json         TEXT,
    payload_digest       TEXT,
    payload_ref          TEXT,
    source_ip            TEXT,
    user_agent           TEXT,
    verification_status  TEXT NOT NULL DEFAULT 'PENDING'
                         CHECK (verification_status IN ('PENDING', 'VERIFIED', 'FAILED')),
    delivery_status      TEXT NOT NULL DEFAULT 'RECEIVED'
                         CHECK (delivery_status IN ('RECEIVED', 'SUCCESS', 'FAILED', 'SKIPPED')),
    failure_code         TEXT,
    failure_message      TEXT,
    received_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    verified_at          TIMESTAMPTZ NULL,
    normalized_event_id  TEXT,
    processed_at         TIMESTAMPTZ NULL,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_webhook_delivery_event
    ON commerce_payment_webhook_delivery (tenant_id, provider_code, event_id);

CREATE INDEX IF NOT EXISTS idx_commerce_payment_webhook_delivery_nonce
    ON commerce_payment_webhook_delivery (tenant_id, provider_code, nonce)
    WHERE nonce IS NOT NULL;
