# SDKWork Payment Backend SDK Component Spec

This component spec declares the generated backend SDK family for `sdkwork-payment`.

- SDK family: `sdkwork-payment-backend-sdk`
- Consumer package: `@sdkwork/payment-backend-sdk`
- API authority: `sdkwork-payment-backend-api`
- API prefix: `/backend/v3/api`
- Languages: TypeScript
- Generator: `../../sdkwork-sdk-generator/bin/sdkgen.js`
- Wire protocol: `sdkwork-v3`
- Auth mode: `dual-token` (`Authorization` bearer JWT + `Access-Token` header)

The component spec records the family identity, dependency boundary, API surface, and wire protocol contract used by the workspace generation checks.

## Verification

- `node --input-type=module -e "import { readFileSync } from 'node:fs'; JSON.parse(readFileSync('specs/component.spec.json','utf8'));"`
- `node sdks/sdkwork-payment-backend-sdk/bin/generate-sdk.mjs`

Run these commands from the `sdkwork-payment` repository root.
