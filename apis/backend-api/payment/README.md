# Payment Backend API

SDKWork-owned backend/admin contract for the `commerce.payment` capability.

- Authority OpenAPI: `sdkwork-payment-backend-api.openapi.yaml`
- API prefix: `/backend/v3/api/payments`
- SDK family: `sdkwork-payment-backend-sdk` → `@sdkwork/payment-backend-sdk`
- Wire protocol: `sdkwork-v3` (default; omitted `x-sdkwork-wire-protocol`)
- Auth: `dual-token` (`AuthToken` bearer JWT + `AccessToken` header)
- Response envelope: `SdkWorkApiResponse` (`code: 0`, `data`, `traceId`) per `API_SPEC.md` §15
- Error envelope: `application/problem+json` (`ProblemDetail`) per `API_SPEC.md` §15.2
- Pagination: `data.items` + `data.pageInfo` per `API_SPEC.md` §16

## Operations

| Resource | Operations |
| --- | --- |
| `payments.intents` | list, retrieve |
| `payments.methods` | list, create, update |
| `payments.providerAccounts` | list, create, update, test, credentials.rotate |
| `payments.channels` | list, create |
| `payments.routeRules` | list, create, update, delete |
| `payments.subMerchants` | list, create, retrieve, update, delete |
| `payments.certificates` | list, create, retrieve, delete |
| `payments.attempts` | list |
| `payments.webhookEvents` | list, replay |
| `payments.reconciliationRuns` | list, create |
| `payments.dev` | sandboxTrigger, webhookSignatureTest |

## Materialization

```bash
node <sdkwork-payment>/scripts/gateway/materialize-payment-backend-openapi.mjs
```

Outputs `sdks/sdkwork-payment-backend-sdk/openapi/sdkwork-payment-backend-api.sdkgen.yaml`.

## Verification

```bash
node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-app-sdk-consumer-imports.mjs --workspace .
```
