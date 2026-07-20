# Payment Technical Architecture

Specs: ARCHITECTURE_DECISION_SPEC.md, DOCUMENTATION_SPEC.md, API_SPEC.md, WEB_FRAMEWORK_SPEC.md, WEB_BACKEND_SPEC.md, SECURITY_SPEC.md, PAGINATION_SPEC.md

Status: active
Owner: SDKWork maintainers
Updated: 2026-07-13

## 1. Architecture Overview

`sdkwork-payment` owns the **payment executor** for the SDKWork commerce domain: payment intents, attempts, owner-order pay side-effects (via order orchestration), refunds, backend admin (methods, providers, channels, webhook event storage, reconciliation). PSP webhooks are **HTTP-owned by sdkwork-order**; payment exposes ingest ports only. Points recharge is **not** in this repository — use `sdkwork-order` (`/app/v3/api/recharges/*`).

**Dependency rule:** `sdkwork-payment` must not take a crate dependency on `sdkwork-order`. Order orchestration calls payment in-process; payment validates `orderId` via read-only SQL in `order_reference.rs` and owns `commerce_payment_method` listing. Shared pay/settlement types are defined in `sdkwork-payment-service`. See `specs/commerce-dependency-boundary.spec.json`.

## Capability stack

| Layer | Path |
| --- | --- |
| Domain contracts (Rust) | `crates/sdkwork-payment-service/` |
| PSP adapters (Stripe/Alipay/WeChat) | `crates/sdkwork-payment-providers/` |
| SQL repositories | `crates/sdkwork-payment-repository-sqlx/` |
| HTTP routers | `crates/sdkwork-routes-payment-app-api/`, `crates/sdkwork-routes-payment-backend-api/` |
| Gateway assembly | `crates/sdkwork-api-payment-assembly/` |
| API server | `crates/sdkwork-api-payment-standalone-gateway/` |
| PC application | `apps/sdkwork-payment-pc/` |
| TypeScript facade | `apps/sdkwork-payment-common/packages/sdkwork-payment-service/` |

### Admin console packages (`apps/sdkwork-payment-pc/packages/`)

| Package | Responsibility |
| --- | --- |
| `sdkwork-payment-pc-admin-core` | Shared infrastructure: `AdminFieldLabel`, `ConfirmDialog`, `CopyButton`, `SdkworkPaymentListPaginationControls`, provider/filter/payment-method constants, coercion helpers, standard exports (sdk/modules/host/session) |
| `sdkwork-payment-pc-admin-provider` | Provider account management: list/create/update/test/rotate credentials + sub-merchant CRUD (Alipay sub_appid / WeChat sub_mch_id / Stripe Connected Account) |
| `sdkwork-payment-pc-admin-channel` | Channel and route rule management: payment methods, channels (scene_code mapping), route rules (priority-based provider selection) |
| `sdkwork-payment-pc-admin-devconfig` | Dev config: certificate CRUD, environment switcher, webhook event integration logs + replay, webhook debugger (sandbox trigger + signature test) |
| `sdkwork-payment-pc-admin-monitor` | Operations monitoring: payment intents, payment attempts, webhook events (with signature status + payload viewer), reconciliation runs |

Each admin package follows the same controller pattern: `createSdkWorkPagedListSession` for paged server-side lists, defensive `map*` projections from `unknown` SDK payloads, and a React-friendly external store (`subscribe` / `getState`). All packages consume `SdkworkPaymentBackendService` via the port-adapter-service pattern (APP_SDK_INTEGRATION_SPEC.md §9); they never import `@sdkwork/payment-backend-sdk` directly.

## API ownership

- App API prefix: `/app/v3/api/payments`, `/app/v3/api/refunds`
- Backend API prefix: `/backend/v3/api/payments`
- Table prefix: `commerce_`

## HTTP contract layer

### SdkWorkApiResponse envelope (`API_SPEC.md` §4.5 / §15 / §16)

All app-api and backend-api success handlers use `api_response.rs` helpers:

- Single resource: `{ "code": 0, "data": { "item": T }, "traceId": "..." }`
- Lists: `{ "code": 0, "data": { "items": [...], "pageInfo": { "mode": "offset", ... } }, "traceId": "..." }`
- Commands: `{ "code": 0, "data": { "accepted": true, "resourceId"?: "...", "status"?: "..." }, "traceId": "..." }`

Errors use HTTP 4xx/5xx `application/problem+json` (`SdkWorkProblemDetail`) with numeric platform `code` and `traceId`. All error helpers set `Content-Type: application/problem+json` explicitly.

### Provider integrations (`sdkwork-payment-providers`)

| Provider | Create | Query | Close | Refund | Webhook verify |
| --- | --- | --- | --- | --- | --- |
| `stripe` | PaymentIntent + `clientSecret` | GET intent | cancel | POST refund | HMAC-SHA256 |
| `alipay` | `trade.precreate` → `qrCodeUrl` | `trade.query` | `trade.close` | `trade.refund` | RSA2 form sign |
| `wechat_pay` | Native → `code_url` | out-trade-no query | close | domestic refund | platform RSA + AES-GCM |

- Registry: tenant-scoped `commerce_payment_provider_account` rows load encrypted, versioned credentials from `commerce_payment_provider_credential`; the adapter receives plaintext only in memory. `PaymentProviderRegistry::from_env()` remains a migration fallback for legacy deployments.
- Routability: catalog methods and channels may be pre-enabled, but a channel bound to a provider account is returned and accepted only while that account is active. Multiple active accounts for the same tenant/organization/provider fail closed until deterministic channel routing is configured.
- Pay flow: after repository persists intent/attempt, shared `enrich_owner_order_payment_*` (`owner_order_checkout.rs`) calls the configured PSP and merges `providerTransactionId` / `providerStatus` into attempt `callback_payload` for later close/cancel.
- Reconcile (app): `POST /payments/reconcile` is a **lookup** command that returns the latest payment record for `orderId` or `outTradeNo`; PSP status repair is not performed inline (use backend webhook replay or order settlement).
- Close: `POST /payments/{paymentId}/close` marks attempt/intent `canceled` in the database first, then best-effort PSP cancel (Stripe uses `providerTransactionId` from attempt `callback_payload` when present).
- Refund: `POST /refunds` persists the refund row, then submits `create_refund` to the PSP with up to three transient retries; terminal PSP failure marks the refund `failed` in DB and returns an error response.
- Checkout polling: `GET /payments/checkout/{paymentId}` re-enriches pending attempts via PSP for cashier/QR parameters.
- Webhook ingest: Payment resolves the attempt by `provider_code` plus `outTradeNo`, applies tenant and organization scope when available, and fails closed when the identity matches more than one attempt. The resolved payment attempt id is carried into Order settlement. Events without resolvable `outTradeNo` are persisted as `unmatched` only when tenant scope is available.
- Sandbox: when `provider_code` is `sandbox` or PSP credentials are absent, local cashier URLs from `sdkwork-utils-rust` are used without external HTTP.

### Provider and async processing

- `SandboxPaymentProvider` remains for contract tests and offline draft generation.
- Backend admin `webhook_events` replay re-applies stored payment attempt status inline; order settlement uses order `payment_confirmations`.

### Webhook replay (admin)

Replay increments `retries` atomically with `COALESCE(retries, 0) < 5`; limit exceeded → 409, missing event → 404. `POST .../webhook_events/{eventId}/replay` requires `Idempotency-Key` and `Sdkwork-Request-Hash`; response uses command envelope (`data.accepted`).

### Payment methods catalog

`GET /payments/methods` joins `commerce_payment_method` with active `commerce_payment_channel.scene_code` values, maps scenes to API `productTypes` (`web` → `pc`, `app`, `mini_program`, `api`), and paginates in SQL (`page`/`page_size`, `data.items` + `pageInfo`). Optional `clientType` filters by channel `scene_code` in the repository layer (not in-process).

### Route manifest

- `sdkwork-routes-payment-app-api/src/http_route_manifest.rs`
- `sdkwork-routes-payment-backend-api/src/http_route_manifest.rs`

Manifests are injected via `WebFrameworkLayer::with_route_manifest`. Idempotent write routes require `Idempotency-Key` and `Sdkwork-Request-Hash` at the handler layer.

### Pagination (`PAGINATION_SPEC.md` §2)

List/search endpoints push `page` / `page_size` to SQL `LIMIT`/`OFFSET` with `COUNT(*) OVER()` (or equivalent aggregate) in the repository layer. Covered paths include payment records, order payments, refunds, backend admin lists, and **app payment methods**. Process-memory `fetch_all` + `skip`/`take` is forbidden on P0 paths.

### Idempotency and transactions

- Owner-order pay: `PayOwnerOrderCommand` carries `idempotency_key` + `request_no`; repository replays by `(tenant_id, order_id, idempotency_key)` and uses deterministic intent/attempt IDs.
- Webhook and confirmation settlement lock records in `order -> payment_intent -> payment_attempt` order. Webhooks confirm the exact resolved attempt; order-only manual confirmation is accepted only when one matching attempt is unambiguous.
- Payment timestamps use UTC RFC3339 at service boundaries. PostgreSQL stores and reads `TIMESTAMPTZ`; SQLite stores the same RFC3339 representation as text. Confirmation replay returns the first persisted non-empty `paid_at`.
- Refunds: idempotency replay + transactional refund-sum guard under `BEGIN IMMEDIATE` (SQLite) / `FOR UPDATE` (PostgreSQL).
- Close / cancel / reconcile: command headers enforced at handler; close is idempotent when record already terminal.
- Domain wire transitions (`validate_payment_wire_transition` / `validate_refund_wire_transition`) enforced on cancel, close, refund create, and owner-order payment confirmation.

### IAM boundary (backend-api)

`backend_runtime_subject_from_extension` enforces organization session, `can_access_backend_api()`, and tenant scope from IAM context (never from URL).

## Data stores

DDL baselines: `database/ddl/baseline/sqlite/` and `database/ddl/baseline/postgres/` — structurally aligned; PostgreSQL uses `NUMERIC`/`TIMESTAMPTZ`/`JSONB`.

## Production hardening

### Legacy PSP environment-variable fallback

| Variable | Provider | Purpose |
| --- | --- | --- |
| `ORDER_PAYMENT_WEBHOOK_BASE_URL` | all | Base URL for `{base}/app/v3/api/orders/payments/webhooks/{providerCode}` notify endpoints (order gateway) |
| `STRIPE_SECRET_KEY` | stripe | API secret |
| `STRIPE_WEBHOOK_SECRET` | stripe | Webhook HMAC verification |
| `ALIPAY_APP_ID` | alipay | Application ID |
| `ALIPAY_PRIVATE_KEY_PEM` | alipay | Merchant RSA private key (PEM) |
| `ALIPAY_PUBLIC_KEY_PEM` | alipay | Alipay RSA public key for response verify |
| `ALIPAY_NOTIFY_URL` | alipay | Optional override notify URL |
| `WECHAT_PAY_MCH_ID` | wechat_pay | Merchant ID |
| `WECHAT_PAY_APP_ID` | wechat_pay | App ID |
| `WECHAT_PAY_API_V3_KEY` | wechat_pay | API v3 key |
| `WECHAT_PAY_MERCHANT_SERIAL_NO` | wechat_pay | Merchant certificate serial |
| `WECHAT_PAY_PRIVATE_KEY_PEM` | wechat_pay | Merchant RSA private key (PEM) |
| `WECHAT_PAY_PLATFORM_PUBLIC_KEY_PEM` | wechat_pay | WeChat platform certificate (PEM) |

### Tenant provider accounts (`commerce_payment_provider_account`)

Backend admin upserts (methods, provider accounts, channels, route rules) and reconciliation run creation use `success_command_accepted` (`data.accepted` + optional `resourceId`). Provider credential inputs are write-only and encrypted before database persistence. At runtime pay/close/refund resolve the active account for `(tenant_id, organization_id, provider_code)`, decrypt its active credential versions, and merge them into the PSP registry.

| Field | Purpose |
| --- | --- |
| `commerce_payment_provider_credential` / `primary_secret` | Encrypted Stripe secret key or Alipay/WeChat merchant private key PEM |
| `commerce_payment_provider_credential` / `webhook_secret` | Encrypted Stripe webhook secret or WeChat API v3 key |
| `commerce_payment_provider_credential` / `certificate` | Encrypted Alipay public key or WeChat platform certificate PEM |
| `merchant_id` | Alipay `app_id` or WeChat `mch_id` |
| `metadata` | JSON extras: `appId`, `merchantSerialNo`, `notifyUrl`, `returnUrl`; production seeds start with explicit mock identifiers that can be replaced without changing adapter code |

WeChat product routing uses `paymentMethod` as the upstream V3 product key. `wechat_jsapi` requires `payerOpenId`; `wechat_h5` requires `clientIp`; Native and App do not require those payer fields. `paymentScene` remains a client/channel scene selector and is not substituted for the provider product key.

Credential envelopes use `PaymentCredentialCipher` with AES-256-GCM and an HKDF context bound to tenant, provider account, and credential kind. The standalone host creates its wrapping key once at `.runtime/payment/credential-master.key`, which is excluded from source control; federated production hosts may install a KMS-backed implementation through `install_payment_credential_cipher`. The wrapping key is never stored in the payment database. Back up or centrally manage that key before running multiple replicas, because losing it makes stored credentials intentionally undecryptable.

- CORS: `PAYMENT_API_CORS_ORIGINS` whitelist (no `*`)
- Graceful shutdown, 30s request timeout, 1 MiB body limit
- Structured tracing via `WebRequestContext` / `x-sdkwork-trace-id`

## Verification

```powershell
cd E:\sdkwork-space\sdkwork-payment
cargo test --workspace
pnpm verify
node ../sdkwork-specs/tools/check-api-response-envelope.mjs --workspace .
node ../sdkwork-specs/tools/check-pagination.mjs --workspace .
```

## Related docs

- PRD: `docs/product/prd/PRD.md`
- Payment executor boundary: `specs/PAYMENT_EXECUTOR_SPEC.md`
- Backend API OpenAPI contract: `apis/backend-api/payment/sdkwork-payment-backend-api.openapi.yaml`
- Commerce migration: `../sdkwork-specs/MIGRATION_SPEC.md` §8
