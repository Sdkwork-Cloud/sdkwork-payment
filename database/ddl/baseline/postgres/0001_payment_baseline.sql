-- sdkwork:baseline
-- module: payment
-- owner: sdkwork-payment
--
-- Payment capability baseline DDL for PostgreSQL.
-- Aligns with sdkwork-specs API_SPEC.md §15, PAGINATION_SPEC.md §2,
-- DATABASE_SPEC.md §3, SECURITY_SPEC.md tenant isolation rules.
--
-- Conventions:
--   * All money columns use NUMERIC(18,2) NOT NULL DEFAULT 0 (no FLOAT/REAL).
--   * All timestamp columns use TIMESTAMPTZ NOT NULL DEFAULT NOW().
--   * All status columns use CHECK constraints with canonical enum values.
--   * All write paths use (tenant_id, organization_id, owner_user_id, ...) composite indexes.
--   * Idempotency columns use UNIQUE constraints to prevent TOCTOU races.
--   * Optimistic locking via version BIGINT NOT NULL DEFAULT 0.
--   * Soft delete via deleted_at TIMESTAMPTZ NULL.

-- =============================================================================
-- 1. commerce_payment_method
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_method (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    organization_id TEXT,
    method_key      TEXT NOT NULL,
    display_name    TEXT NOT NULL,
    provider_code   TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'inactive', 'deprecated')),
    sort_order      INTEGER NOT NULL DEFAULT 0,
    scope           TEXT NOT NULL DEFAULT 'tenant'
                    CHECK (scope IN ('global', 'tenant', 'organization')),
    currency_code   TEXT NOT NULL DEFAULT 'CNY',
    country_code    TEXT,
    metadata        JSONB NOT NULL DEFAULT '{}'::jsonb,
    request_no      TEXT,
    idempotency_key TEXT NOT NULL,
    version         BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_method_tenant_org_key
    ON commerce_payment_method (tenant_id, COALESCE(organization_id, ''), method_key)
    WHERE deleted_at IS NULL;

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_method_idempotency
    ON commerce_payment_method (tenant_id, COALESCE(organization_id, ''), idempotency_key)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_method_lookup
    ON commerce_payment_method (tenant_id, method_key, status)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_method_tenant_org
    ON commerce_payment_method (tenant_id, organization_id, status, sort_order)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 2. commerce_payment_intent
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_intent (
    id                TEXT PRIMARY KEY,
    tenant_id         TEXT NOT NULL,
    organization_id   TEXT,
    owner_user_id     TEXT NOT NULL,
    order_id          TEXT NOT NULL,
    payment_intent_no TEXT NOT NULL,
    payment_method    TEXT NOT NULL,
    provider_code     TEXT NOT NULL,
    amount            NUMERIC(18,2) NOT NULL DEFAULT 0,
    currency_code     TEXT NOT NULL DEFAULT 'CNY',
    status            TEXT NOT NULL DEFAULT 'pending'
                      CHECK (status IN ('created', 'pending', 'processing', 'succeeded', 'failed', 'canceled', 'closed')),
    request_no        TEXT,
    idempotency_key   TEXT NOT NULL,
    version           BIGINT NOT NULL DEFAULT 0,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_intent_idempotency
    ON commerce_payment_intent (tenant_id, order_id, idempotency_key)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_intent_owner
    ON commerce_payment_intent (tenant_id, owner_user_id, id)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_intent_owner_order
    ON commerce_payment_intent (tenant_id, owner_user_id, order_id, status)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_intent_created
    ON commerce_payment_intent (tenant_id, owner_user_id, created_at DESC, id DESC)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 3. commerce_payment_attempt
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_attempt (
    id                     TEXT PRIMARY KEY,
    tenant_id              TEXT NOT NULL,
    organization_id        TEXT,
    owner_user_id          TEXT NOT NULL,
    payment_intent_id      TEXT NOT NULL,
    order_id               TEXT NOT NULL,
    attempt_no             TEXT,
    payment_method         TEXT NOT NULL,
    provider_code          TEXT NOT NULL,
    channel_id             TEXT,
    out_trade_no           TEXT,
    amount                 NUMERIC(18,2) NOT NULL DEFAULT 0,
    currency_code          TEXT NOT NULL DEFAULT 'CNY',
    status                 TEXT NOT NULL DEFAULT 'pending'
                           CHECK (status IN ('created', 'pending', 'processing', 'succeeded', 'failed', 'canceled', 'closed')),
    provider_transaction_id TEXT,
    callback_payload       JSONB NOT NULL DEFAULT '{}'::jsonb,
    paid_at                TIMESTAMPTZ NULL,
    request_no             TEXT,
    idempotency_key        TEXT NOT NULL,
    version                BIGINT NOT NULL DEFAULT 0,
    created_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at             TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_attempt_idempotency
    ON commerce_payment_attempt (tenant_id, owner_user_id, payment_intent_id, id)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_attempt_owner_intent
    ON commerce_payment_attempt (tenant_id, owner_user_id, payment_intent_id, status)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_attempt_owner_order
    ON commerce_payment_attempt (tenant_id, owner_user_id, order_id, status, created_at DESC, id DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_attempt_out_trade_no
    ON commerce_payment_attempt (tenant_id, out_trade_no)
    WHERE out_trade_no IS NOT NULL AND deleted_at IS NULL;

-- =============================================================================
-- 4. commerce_refund
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_refund (
    id                  TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    organization_id     TEXT,
    order_id            TEXT NOT NULL,
    payment_attempt_id  TEXT NOT NULL,
    refund_no           TEXT NOT NULL,
    amount              NUMERIC(18,2) NOT NULL DEFAULT 0,
    currency_code       TEXT NOT NULL DEFAULT 'CNY',
    status              TEXT NOT NULL DEFAULT 'submitted'
                        CHECK (status IN ('submitted', 'processing', 'succeeded', 'failed', 'canceled')),
    refund_reason_code  TEXT,
    requested_by_type    TEXT NOT NULL DEFAULT 'buyer'
                        CHECK (requested_by_type IN ('buyer', 'operator', 'system')),
    requested_by        TEXT,
    request_no          TEXT,
    idempotency_key     TEXT NOT NULL,
    version             BIGINT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_refund_idempotency
    ON commerce_refund (tenant_id, order_id, idempotency_key)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_refund_owner
    ON commerce_refund (tenant_id, organization_id, order_id, status, created_at DESC, id DESC)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_refund_attempt
    ON commerce_refund (tenant_id, payment_attempt_id, status)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 5. commerce_refund_event
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_refund_event (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    organization_id TEXT,
    event_no        TEXT NOT NULL,
    refund_id       TEXT NOT NULL,
    event_type      TEXT NOT NULL
                    CHECK (event_type IN ('created', 'status_changed', 'succeeded', 'failed', 'canceled')),
    from_status     TEXT,
    to_status       TEXT NOT NULL,
    actor_type      TEXT NOT NULL DEFAULT 'buyer'
                    CHECK (actor_type IN ('buyer', 'operator', 'system')),
    actor_id        TEXT,
    request_id      TEXT,
    idempotency_key TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_refund_event_idempotency
    ON commerce_refund_event (tenant_id, refund_id, event_type, idempotency_key);

CREATE INDEX IF NOT EXISTS idx_commerce_refund_event_refund
    ON commerce_refund_event (tenant_id, refund_id, created_at DESC);

-- =============================================================================
-- 6. commerce_payment_channel
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_channel (
    id                  TEXT PRIMARY KEY,
    tenant_id           TEXT NOT NULL,
    organization_id    TEXT,
    channel_no          TEXT NOT NULL,
    channel_name        TEXT,
    provider_account_id TEXT,
    method_id           TEXT,
    provider_code       TEXT NOT NULL,
    scene_code          TEXT NOT NULL DEFAULT 'app'
                        CHECK (scene_code IN ('app', 'web', 'mini_program', 'api')),
    currency_code       TEXT NOT NULL DEFAULT 'CNY',
    country_code        TEXT,
    status              TEXT NOT NULL DEFAULT 'active'
                        CHECK (status IN ('active', 'inactive', 'deprecated')),
    priority            INTEGER NOT NULL DEFAULT 0,
    sort_order          INTEGER NOT NULL DEFAULT 0,
    metadata            JSONB NOT NULL DEFAULT '{}'::jsonb,
    version             BIGINT NOT NULL DEFAULT 0,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at          TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_channel_tenant_org_no
    ON commerce_payment_channel (tenant_id, COALESCE(organization_id, ''), channel_no)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_channel_provider
    ON commerce_payment_channel (tenant_id, provider_code, status)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_channel_method
    ON commerce_payment_channel (tenant_id, method_id, scene_code, status)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 7. commerce_payment_provider_account
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_provider_account (
    id                 TEXT PRIMARY KEY,
    tenant_id          TEXT NOT NULL,
    organization_id   TEXT,
    account_no         TEXT NOT NULL,
    provider_code      TEXT NOT NULL,
    merchant_id        TEXT,
    environment        TEXT NOT NULL DEFAULT 'production'
                       CHECK (environment IN ('development', 'sandbox', 'production')),
    country_code       TEXT,
    settlement_currency TEXT NOT NULL DEFAULT 'CNY',
    secret_ref         TEXT NOT NULL,
    webhook_secret_ref TEXT,
    certificate_ref    TEXT,
    status             TEXT NOT NULL DEFAULT 'active'
                       CHECK (status IN ('active', 'inactive', 'suspended', 'deprecated')),
    metadata           JSONB NOT NULL DEFAULT '{}'::jsonb,
    version            BIGINT NOT NULL DEFAULT 0,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at         TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_provider_account_tenant_org_no
    ON commerce_payment_provider_account (tenant_id, COALESCE(organization_id, ''), account_no)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_provider_account_status
    ON commerce_payment_provider_account (tenant_id, provider_code, status)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 8. commerce_payment_route_rule
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_route_rule (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    organization_id TEXT,
    rule_no         TEXT NOT NULL,
    priority        INTEGER NOT NULL DEFAULT 0,
    purchase_type   TEXT,
    country_code    TEXT,
    currency_code   TEXT,
    client_platform TEXT,
    amount_min      NUMERIC(18,2),
    amount_max      NUMERIC(18,2),
    user_segment    TEXT,
    risk_level      TEXT,
    channel_id      TEXT NOT NULL,
    status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'inactive', 'deprecated')),
    starts_at       TIMESTAMPTZ,
    ends_at         TIMESTAMPTZ,
    version         BIGINT NOT NULL DEFAULT 0,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at      TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_route_rule_tenant_org_no
    ON commerce_payment_route_rule (tenant_id, COALESCE(organization_id, ''), rule_no)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_route_rule_lookup
    ON commerce_payment_route_rule (tenant_id, organization_id, status, priority)
    WHERE deleted_at IS NULL;

-- =============================================================================
-- 9. commerce_payment_webhook_event
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_webhook_event (
    id              TEXT PRIMARY KEY,
    tenant_id       TEXT NOT NULL,
    organization_id TEXT,
    event_id        TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    provider_code   TEXT,
    payload         JSONB NOT NULL DEFAULT '{}'::jsonb,
    status          TEXT NOT NULL DEFAULT 'queued'
                    CHECK (status IN ('queued', 'processing', 'processed', 'failed', 'dead')),
    retries         INTEGER NOT NULL DEFAULT 0,
    last_error      TEXT,
    received_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    processed_at    TIMESTAMPTZ NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_webhook_event_event_id
    ON commerce_payment_webhook_event (tenant_id, event_id);

CREATE INDEX IF NOT EXISTS idx_commerce_payment_webhook_event_status
    ON commerce_payment_webhook_event (tenant_id, status, received_at)
    WHERE status IN ('queued', 'processing', 'failed');

-- =============================================================================
-- 10. commerce_payment_reconciliation_run
-- =============================================================================
CREATE TABLE IF NOT EXISTS commerce_payment_reconciliation_run (
    id                      TEXT PRIMARY KEY,
    tenant_id               TEXT NOT NULL,
    organization_id         TEXT,
    run_no                  TEXT NOT NULL,
    provider_code           TEXT,
    provider_account_id     TEXT,
    reconciliation_type     TEXT NOT NULL DEFAULT 'daily'
                            CHECK (reconciliation_type IN ('daily', 'weekly', 'monthly', 'manual', 'settlement')),
    period_start            TIMESTAMPTZ NOT NULL,
    period_end              TIMESTAMPTZ NOT NULL,
    status                  TEXT NOT NULL DEFAULT 'pending'
                            CHECK (status IN ('pending', 'queued', 'running', 'succeeded', 'failed', 'canceled')),
    matched_count           INTEGER NOT NULL DEFAULT 0,
    mismatched_count        INTEGER NOT NULL DEFAULT 0,
    unmatched_count         INTEGER NOT NULL DEFAULT 0,
    total_difference_amount NUMERIC(18,2) NOT NULL DEFAULT 0,
    currency_code           TEXT NOT NULL DEFAULT 'CNY',
    request_no              TEXT,
    idempotency_key         TEXT NOT NULL,
    version                 BIGINT NOT NULL DEFAULT 0,
    created_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at              TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at              TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_reconciliation_run_no
    ON commerce_payment_reconciliation_run (tenant_id, run_no)
    WHERE deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_reconciliation_run_period
    ON commerce_payment_reconciliation_run (tenant_id, provider_code, period_start DESC)
    WHERE deleted_at IS NULL;
