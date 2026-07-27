# SDKWork Payment SQLx Repository Component Specs

This directory defines the local component contract for `sdkwork-payment-repository-sqlx`.

- Component root: `sdkwork-payment/crates/sdkwork-payment-repository-sqlx`
- Machine contract: `specs/component.spec.json`
- Public boundary: SQLite and PostgreSQL payment repositories exported by the crate root
- Concurrency boundary: repository-controlled transactions, per-Order checkout serialization, and provider-trade uniqueness
- Verification: `cargo test -p sdkwork-payment-repository-sqlx`

Root standards under `../../../sdkwork-specs/` remain authoritative. This component does not own Order business identity, PSP protocol implementations, HTTP routes, or generated SDKs.
