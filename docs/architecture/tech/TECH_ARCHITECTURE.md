# Payment Technical Architecture

Status: active
Owner: SDKWork maintainers
Updated: 2026-06-24

## Capability stack

`sdkwork-payment` owns the full **payment** capability:

| Layer | Path |
| --- | --- |
| Domain (Rust) | `crates/sdkwork-commerce-payment-service/` |
| SQL | `crates/sdkwork-commerce-payment-repository-sqlx/` |
| HTTP routers | `crates/sdkwork-router-payment-*-api/` |
| API server | `crates/sdkwork-payment-api-server/` |
| PC client | `apps/sdkwork-payment-pc/` |
| Client facade | `packages/common/payment/sdkwork-payment-service/` |

## PC surface

```text
apps/sdkwork-payment-pc/
  packages/sdkwork-payment-pc-core/
  packages/sdkwork-payment-pc-shell/
  packages/sdkwork-payment-pc-payment/    ← migrated from sdkwork-commerce-pc
```

Composition apps (`sdkwork-mall`, etc.) consume `@sdkwork/payment-pc-payment` via workspace paths — not a central commerce PC repo.

## API ownership

- App API prefix: `/app/v3/api/payments`
- Backend API prefix: `/backend/v3/api/payments`
- Table prefix: `commerce_` (commerce domain)

## Verification

```powershell
cd E:\sdkwork-space\sdkwork-payment
pnpm verify
```

## Related docs

- [Commerce PC distribution](../../../sdkwork-commerce/docs/architecture/tech/TECH-2026-06-24-commerce-pc-capability-distribution.md)
- [Commerce repository dissolution](../../../sdkwork-commerce/docs/architecture/tech/TECH-2026-06-24-commerce-repository-dissolution.md)
