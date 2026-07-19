CREATE TABLE IF NOT EXISTS commerce_payment_provider_credential (
    id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    organization_id TEXT,
    provider_account_id TEXT NOT NULL,
    credential_kind TEXT NOT NULL CHECK (credential_kind IN ('primary_secret', 'webhook_secret', 'certificate')),
    ciphertext TEXT NOT NULL,
    encryption_key_id TEXT NOT NULL,
    encryption_algorithm TEXT NOT NULL,
    fingerprint_sha256 TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'superseded', 'revoked')),
    version BIGINT NOT NULL DEFAULT 1,
    rotated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    deleted_at TIMESTAMPTZ,
    CONSTRAINT fk_commerce_payment_provider_credential_account
        FOREIGN KEY (provider_account_id) REFERENCES commerce_payment_provider_account(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_commerce_payment_provider_credential_active
    ON commerce_payment_provider_credential
       (tenant_id, COALESCE(organization_id, ''), provider_account_id, credential_kind)
    WHERE status = 'active' AND deleted_at IS NULL;

CREATE INDEX IF NOT EXISTS idx_commerce_payment_provider_credential_history
    ON commerce_payment_provider_credential
       (tenant_id, provider_account_id, credential_kind, version DESC);
