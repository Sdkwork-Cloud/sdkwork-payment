# Common Seed Scripts

Locale-agnostic payment bootstrap data. The seed manifest explicitly selects one
of the following environment profiles; directory ordering is never used.

For CLI-driven local setup, select the profile explicitly through
`pnpm db:seed:dev` or `pnpm db:bootstrap:dev`. The unsuffixed CLI commands use
the `standard` profile; `SDKWORK_PAYMENT_DATABASE_SEED_PROFILE` is consumed by
embedded service startup rather than overriding the CLI subcommand default.

- `development`: complete payment catalog, an active local sandbox channel, and
  organization-scoped demo records covering the full admin workflow. Demo rows
  contain no usable credentials or private certificate material.
- `test`: complete payment catalog plus an active isolated test sandbox channel.
- `production` / `standard`: complete payment catalog and editable PSP templates.
  Catalog methods and channels are pre-wired and active, but remain hidden from
  payment routing until their provider account is active. Provider accounts
  remain inactive and contain migration-compatible mock references only. The templates
  include provider-specific mock identifiers and metadata (for example
  `mock-wechat-mch-id`, `mock-wechat-app-id`, and
  `mock-wechat-merchant-serial-no`). Replace those values and the referenced
  database-backed write-only credentials, then activate the account; no schema, method, or
  adapter code changes are required for a live WeChat Pay connection.

All records are scoped to the platform bootstrap tenant `100001`. Backend-admin
records use the stable bootstrap administrator organization `100002`; IAM
organization id `0` is a tenant-login sentinel and is not a valid organization
session scope. Catalog/template scripts insert only missing business records.
`006_upgrade_bootstrap_templates.sql` repairs only bootstrap-marked rows, so
real administrator-owned configurations are not overwritten.

Keep JSON literals inside the target table's `INSERT ... VALUES` context. Moving
them into an untyped CTE makes PostgreSQL infer `text`, which cannot be assigned
to the payment tables' `jsonb` columns; target-context values remain portable to
SQLite's TEXT-backed JSON fields.

No seed contains merchant credentials, certificate material, API keys, webhook
secrets, or private keys. Operators replace the mock identifiers and write-only
credentials in the generated provider-account records from the payment admin
workspace (or by editing the seed before first bootstrap) before enabling a
live channel. For WeChat Pay, the merchant private-key PEM, API v3 key, and
platform certificate PEM are encrypted into versioned
`commerce_payment_provider_credential` rows.

Activation is intentionally a second, status-only update. Save the account as
`inactive`, run the backend provider-account dry-run, then set `status` to
`active`. The backend rejects stale tests, remaining mock markers, and
activation requests that also change credentials or merchant identifiers.
