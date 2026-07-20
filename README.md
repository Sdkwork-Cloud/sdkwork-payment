# sdkwork-payment
repository-kind: application

SDKWork commerce **payment** capability building-block repository (domain `commerce`).

- Standards: `../sdkwork-specs/README.md`
- Domain service: `crates/sdkwork-payment-service/`
- Repository SQL: `crates/sdkwork-payment-repository-sqlx/`
- PSP adapters: `crates/sdkwork-payment-providers/`
- HTTP API server: `crates/sdkwork-api-payment-standalone-gateway/`
- PC application: `apps/sdkwork-payment-pc/` (checkout + admin console)

## Quick start

```bash
cargo test --workspace
```

## Documentation Canon

- [docs/README.md](docs/README.md)
- [docs/product/prd/PRD.md](docs/product/prd/PRD.md)
- [docs/architecture/tech/TECH_ARCHITECTURE.md](docs/architecture/tech/TECH_ARCHITECTURE.md)

## Application Roots

- [apps directory index](apps/README.md)
