# REQ-2026-0001 Payment Account And Refund Operations

id: REQ-2026-0001
title: Payment account configuration and secure refund operations
owner: SDKWork payment maintainers
status: in-progress
source: operator
problem: Payment provider account setup is presented as several equally important configuration areas, while payment refunds have no backend-admin API or operator workflow even though buyer refunds and persistence already exist.

## Goals

- Make a provider account the primary payment configuration object; keep payment methods, channels, route rules, sub-merchants, certificates, and diagnostics as progressively disclosed advanced configuration.
- Let payment operators list and inspect tenant-scoped refunds from the Payment Center.
- Let authorized operators initiate a full or partial refund from a succeeded payment intent.
- Preserve the original successful payment attempt and provider account for every provider refund.
- Make refund submission retry-safe, auditable, and explicit about provider-processing and failure states.

## Non-Goals

- Replacing order-domain after-sales approval or refund-request review.
- Allowing an operator to select a different provider account from the original payment.
- Introducing a new refund database table or migration in this requirement.
- Treating frontend permission checks as authorization enforcement.
- Marking an asynchronous provider refund as succeeded before provider evidence is received.

## Users

- Payment operator
- Finance operator
- Reconciliation staff
- Platform administrator

## Acceptance Criteria

- Payment Center names and presents provider records as payment accounts and makes account setup the first configuration step.
- Backend-admin exposes paginated refund list and retrieve operations scoped by authenticated tenant and organization context.
- Backend-admin refund creation accepts a payment intent id, optional partial amount, and required reason; it derives order, owner, attempt, currency, and provider account from authoritative payment data.
- The store locks the order and rejects unpaid orders, amounts above the remaining refundable balance, currency mismatches, and attempts that did not succeed.
- Refund creation requires command idempotency metadata and records `requested_by_type=operator`, the authenticated operator id, request identity, and a refund event.
- Provider submission always reuses the original payment attempt and provider account. Historical inactive or deprecated accounts remain eligible to discharge refund obligations; missing, suspended, deleted, or refund-incompatible account configuration fails closed.
- A provider-accepted refund transitions from `submitted` to `processing`; provider failure transitions it to `failed`.
- Only failed refunds can be retried, and retries reuse the existing refund number so provider idempotency is preserved.
- Manager permissions independently gate refund read, create, and retry actions; backend route authorization remains authoritative.
- Payment records offer a direct refund action, while a dedicated refund center exposes filters, status, amount, reason, requester, and retry state.
- OpenAPI, route manifest, generated backend SDK, service method tree, and frontend controller expose matching refund operations without raw HTTP or generated-file edits.

## Non-Functional Requirements

- Security: dual-token backend authentication, organization-scoped data access, least-privilege permissions, command idempotency, request fingerprint validation, credential redaction, and operator audit attribution.
- Privacy: expose only payment and operator identifiers required for refund operations; never expose provider secrets.
- Performance: use server-side offset pagination with a default page size of 20 and maximum of 200.
- Reliability: retry transient provider submission at the existing bounded retry policy; never create a second refund row when retrying an existing failed refund.

## Affected Surfaces

- api
- sdk
- backend
- pc
- composition

## Trace

### Specs

- `REQUIREMENTS_SPEC.md`
- `API_SPEC.md`
- `PAGINATION_SPEC.md`
- `SECURITY_SPEC.md`
- `SDK_SPEC.md`
- `APP_SDK_INTEGRATION_SPEC.md`
- `BACKEND_UI_SPEC.md`

### Components

- `crates/sdkwork-routes-payment-backend-api`
- `crates/sdkwork-payment-repository-sqlx`
- `sdks/sdkwork-payment-backend-sdk`
- `apps/sdkwork-payment-common/packages/sdkwork-payment-sdk-ports`
- `apps/sdkwork-payment-pc/packages/sdkwork-payment-pc-admin-monitor`
- `../sdkwork-manager/apps/sdkwork-manager-pc/packages/sdkwork-manager-pc-admin-payment`

## Verification

- `cargo test -p sdkwork-payment-service -p sdkwork-payment-repository-sqlx -p sdkwork-routes-payment-backend-api`
- `pnpm --filter @sdkwork/payment-pc-admin-monitor typecheck`
- `pnpm exec vitest run apps/sdkwork-payment-pc/packages/sdkwork-payment-pc-admin-monitor/tests --config vitest.config.ts --configLoader native --pool vmThreads`
- `pnpm sdk:check`
- `node ../sdkwork-specs/tools/check-api-operation-patterns.mjs --workspace .`
- `node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .`
- `node ../sdkwork-specs/tools/check-pagination.mjs --workspace .`
- `node ../sdkwork-specs/tools/check-app-sdk-consumer-imports.mjs --workspace .`
