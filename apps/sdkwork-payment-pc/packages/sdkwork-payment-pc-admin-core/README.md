# @sdkwork/payment-pc-admin-core

Payment PC `backend-admin` runtime package per `APP_PC_ARCHITECTURE_SPEC.md` §3.

Owns backend SDK inventory, admin capability module registry, and composition metadata for `@sdkwork/payment-pc-admin-*` packages. Does not own business pages or UI shells.

## Verification

```bash
pnpm --filter @sdkwork/payment-pc-admin-core typecheck
```
