# SDKWork Payment Backend SDK

This SDK family is generated from the `sdkwork-payment-backend-api` authority contract for `/backend/v3/api`.

The authored OpenAPI spec lives at `apis/backend-api/payment/sdkwork-payment-backend-api.openapi.yaml` and is materialized into this family under `openapi/sdkwork-payment-backend-api.openapi.yaml`. The sdkgen input (`openapi/sdkwork-payment-backend-api.sdkgen.yaml`) mirrors the authority spec until a dedicated materialization tool is added for payment.

## Contract

- SDK family: `sdkwork-payment-backend-sdk`
- API authority: `sdkwork-payment-backend-api`
- API prefix: `/backend/v3/api`
- Audience: backend consoles, finance operators, payment integrators, and reconciliation staff
- Auth mode: `Authorization: Bearer <auth_token>` plus `Access-Token: <access_token>`
- Request context: server middleware resolves `WebRequestContext`; clients must not send `X-Request-Id`
- Wire protocol: `sdkwork-v3` (default; omitted `x-sdkwork-wire-protocol`)
- Response envelope: `SdkWorkApiResponse` (`code: 0`, `data`, `traceId`) per `API_SPEC.md` §15
- Error envelope: `application/problem+json` (`ProblemDetail`) per `API_SPEC.md` §15.2
- Pagination: `data.items` + `data.pageInfo` per `API_SPEC.md` §16

Client responsibilities:

- Construct the generated backend SDK through backend/admin bootstrap.
- Set `auth_token` and `access_token` through generated SDK auth/bootstrap APIs.
- Call typed resource methods generated from `tag + dotted operationId`.
- Never parse tokens for tenant, organization, user, operator, or permission decisions.
- Never generate or send `X-Request-Id`.
- Never replace a missing backend method with raw HTTP.

## Languages

This family ships TypeScript only (no multi-language surface):

- `sdkwork-payment-backend-sdk-typescript` → `@sdkwork/payment-backend-sdk`

## Materialization Flow

1. Author the OpenAPI contract at `apis/backend-api/payment/sdkwork-payment-backend-api.openapi.yaml`.
2. Mirror it into the family `openapi/` directory as both the authority spec and sdkgen input.
3. Run the generator (see below) to materialize `sdkwork-payment-backend-sdk-typescript/generated/server-openapi/`.
4. The composed facade at `sdkwork-payment-backend-sdk-typescript/src/index.ts` re-exports from the generated transport.

## Generation

Run from the `sdkwork-payment` workspace root:

```bash
node sdks/sdkwork-payment-backend-sdk/bin/generate-sdk.mjs
```

Or call the platform-specific script directly:

```bash
# Windows PowerShell
pwsh -File sdks/sdkwork-payment-backend-sdk/bin/generate-sdk.ps1

# Bash
bash sdks/sdkwork-payment-backend-sdk/bin/generate-sdk.sh
```

The wrapper invokes `../../sdkwork-sdk-generator/bin/sdkgen.js` with `--standard-profile sdkwork-v3 -t backend`.

The `generated/server-openapi/` directory does **not** exist until the generator runs. The scripts fail fast with a clear message if the canonical generator (`../../sdkwork-sdk-generator/bin/sdkgen.js`) is not present in the workspace.

## SDKWork Documentation Contract

Domain: commerce
Capability: payment
Package type: sdk-family
Status: standard

### Public API

Public exports are declared in `specs/component.spec.json` under `contracts`.

### Configuration

Configuration keys and runtime entrypoints are declared in `specs/component.spec.json`.

### Security

Do not add secrets, live tokens, manual auth headers, or app-local credential handling to this module.

### Verification

- `pnpm run verify`
- `node sdks/sdkwork-payment-backend-sdk/bin/generate-sdk.mjs`

### Owner And Status

Owner and lifecycle status are tracked in `specs/component.spec.json`.
