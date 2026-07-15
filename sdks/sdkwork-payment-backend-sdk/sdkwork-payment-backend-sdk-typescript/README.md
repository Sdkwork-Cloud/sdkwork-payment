# SDKWork Payment Backend SDK (TypeScript)

This package is the composed consumer facade for the `sdkwork-payment-backend-sdk` family. It re-exports the generated TypeScript transport from `./generated/server-openapi/src/index` so application, feature, shell, and service packages can consume `@sdkwork/payment-backend-sdk` without importing generator transport package names.

- Consumer package: `@sdkwork/payment-backend-sdk`
- Transport package (generator-owned): `sdkwork-payment-backend-sdk-generated-typescript`
- Composed entry: `src/index.ts`
- Transport entry: `generated/server-openapi/src/index.ts`

## Generation

The `generated/server-openapi/` directory is produced by the canonical SDK generator and does **not** exist until generation runs. To materialize it:

```bash
# Cross-platform
node ../bin/generate-sdk.mjs

# Windows PowerShell
pwsh -File ../bin/generate-sdk.ps1

# Bash
bash ../bin/generate-sdk.sh
```

The scripts fail fast if `../../sdkwork-sdk-generator/bin/sdkgen.js` is not present in the workspace. The generator is invoked with `--standard-profile sdkwork-v3 -t backend`.

After generation, the facade at `src/index.ts` re-exports the client constructor, types, API, HTTP, and auth surfaces from the generated transport.

## Consumption

Import only from the composed facade:

```typescript
import { createClient, type SdkworkBackendClient } from '@sdkwork/payment-backend-sdk';
```

Do not import from `sdkwork-payment-backend-sdk-generated-typescript` or deep `generated/server-openapi/src/*` paths from consumer code.
