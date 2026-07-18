# Common Seed Scripts

Locale-agnostic payment bootstrap data. The seed manifest explicitly selects one
of the following environment profiles; directory ordering is never used.

- `development`: complete payment catalog plus an active local sandbox channel.
- `test`: complete payment catalog plus an active isolated test sandbox channel.
- `production` / `standard`: complete payment catalog and editable PSP templates,
  all inactive and with environment-variable references only.

All records are scoped to the platform bootstrap tenant `100001` and organization
`0`, matching the shared commerce bootstrap scope. Scripts insert only missing
business records, so a later seed run does not overwrite administrator changes.

No seed contains merchant credentials, certificate material, API keys, webhook
secrets, or private keys. Operators configure the generated provider-account
records from the payment admin workspace before enabling a live channel.
