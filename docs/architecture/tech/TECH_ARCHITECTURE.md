# Payment Technical Architecture

Specs: ARCHITECTURE_DECISION_SPEC.md, DOCUMENTATION_SPEC.md, API_SPEC.md, WEB_FRAMEWORK_SPEC.md, WEB_BACKEND_SPEC.md, SECURITY_SPEC.md, PAGINATION_SPEC.md

Status: active
Owner: SDKWork maintainers
Updated: 2026-07-06

## 1. Architecture Overview

`sdkwork-payment` owns the **payment executor** for the SDKWork commerce domain: payment intents, attempts, owner-order pay side-effects (via order orchestration), refunds, backend admin (methods, providers, channels, webhook event storage, reconciliation). PSP webhooks are **HTTP-owned by sdkwork-order**; payment exposes ingest ports only. Deprecated recharge HTTP proxy is opt-in only.

## Capability stack

| Layer | Path |
| --- | --- |
| Domain contracts (Rust) | `crates/sdkwork-payment-service/` |
| PSP adapters (Stripe/Alipay/WeChat) | `crates/sdkwork-payment-providers/` |
| SQL repositories | `crates/sdkwork-payment-repository-sqlx/` |
| HTTP routers | `crates/sdkwork-routes-payment-app-api/`, `crates/sdkwork-routes-payment-backend-api/` |
| Gateway assembly | `crates/sdkwork-payment-gateway-assembly/` |
| API server | `crates/sdkwork-payment-standalone-gateway/` |
| PC client | `apps/sdkwork-payment-pc/` |
| TypeScript facade | `apps/sdkwork-payment-common/packages/sdkwork-payment-service/` |

## API ownership

- App API prefix: `/app/v3/api/payments` (deprecated `/app/v3/api/recharges/*` proxy opt-in only)
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

- Registry: `PaymentProviderRegistry::from_env()` reads deployment env vars; tenant-scoped `commerce_payment_provider_account` rows override per provider via `secret_ref` (env var name), `webhook_secret_ref`, `certificate_ref`, and `metadata` JSON.
- Pay flow: after repository persists intent/attempt, `enrich_pay_owner_order_outcome` calls the configured PSP and returns real cashier params.
- Close: `POST /payments/{paymentId}/close` cancels the PSP intent (when configured) before marking attempt/intent `canceled` in the database.
- Refund: `POST /refunds` persists the refund row, then submits `create_refund` to the PSP using the linked payment attempt `out_trade_no`.
- Webhook: order gateway `POST /app/v3/api/orders/payments/webhooks/{providerCode}` calls payment ingest ports; legacy payment path returns 410 Gone.
- Sandbox: when `provider_code` is `sandbox` or PSP credentials are absent, local cashier URLs from `sdkwork-utils-rust` are used without external HTTP.

### Provider and async processing

- `SandboxPaymentProvider` remains for contract tests and offline draft generation.
- `process_queued_webhook_events` (`sqlite_webhook_worker.rs`) drains `queued` rows for admin replay; returns `payment_attempt_contexts` for the **order** gateway to settle in-process.

### Webhook replay (admin)

Replay increments `retries` atomically with `COALESCE(retries, 0) < 5`; limit exceeded → 409, missing event → 404.

### Payment methods catalog

`GET /payments/methods` joins `commerce_payment_method` with active `commerce_payment_channel.scene_code` values and maps scenes to API `productTypes` (`web` → `pc`, `app`, `mini_program`, `api`). Optional `clientType` filters by available product type codes.

### Route manifest

- `sdkwork-routes-payment-app-api/src/http_route_manifest.rs`
- `sdkwork-routes-payment-backend-api/src/http_route_manifest.rs`

Manifests are injected via `WebFrameworkLayer::with_route_manifest`. Idempotent write routes require `Idempotency-Key` and `Sdkwork-Request-Hash` at the handler layer.

### Pagination (`PAGINATION_SPEC.md` §2)

List/search endpoints push `page` / `pageSize` to SQL `LIMIT`/`OFFSET` with `COUNT(*) OVER()` (or equivalent aggregate) in the repository layer. Process-memory `fetch_all` + `skip`/`take` is forbidden on P0 paths.

### Idempotency and transactions

- Owner-order pay: `PayOwnerOrderCommand` carries `idempotency_key` + `request_no`; repository replays by `(tenant_id, order_id, idempotency_key)` and uses deterministic intent/attempt IDs.
- Refunds: idempotency replay + transactional refund-sum guard under `BEGIN IMMEDIATE` (SQLite) / `FOR UPDATE` (PostgreSQL).
- Close / cancel / reconcile: command headers enforced at handler; close is idempotent when record already terminal.
- Domain wire transitions (`validate_payment_wire_transition` / `validate_refund_wire_transition`) enforced on cancel, close, refund create, and owner-order payment confirmation.

### IAM boundary (backend-api)

`backend_runtime_subject_from_extension` enforces organization session, `can_access_backend_api()`, and tenant scope from IAM context (never from URL).

## Data stores

DDL baselines: `database/ddl/baseline/sqlite/` and `database/ddl/baseline/postgres/` — structurally aligned; PostgreSQL uses `NUMERIC`/`TIMESTAMPTZ`/`JSONB`.

## Production hardening

### PSP environment variables

| Variable | Provider | Purpose |
| --- | --- | --- |
| `PAYMENT_WEBHOOK_BASE_URL` | all | Base URL for `{base}/app/v3/api/payments/webhooks/{providerCode}` notify endpoints |
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

Backend admin upserts provider accounts with `secretRef` pointing to an **environment variable name** (never plaintext secrets in DB). At runtime pay/close/refund resolve the active account for `(tenant_id, organization_id, provider_code)` and merge credentials into the PSP registry.

| Field | Purpose |
| --- | --- |
| `secret_ref` | Env var for primary secret (Stripe secret key, Alipay/WeChat private key PEM) |
| `webhook_secret_ref` | Env var for webhook secret (Stripe) or WeChat API v3 key |
| `certificate_ref` | Env var for Alipay public key or WeChat platform cert PEM |
| `merchant_id` | Alipay `app_id` or WeChat `mch_id` |
| `metadata` | JSON extras: `appId`, `merchantSerialNo`, `returnUrl` |

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
- Commerce migration: `../sdkwork-specs/MIGRATION_SPEC.md` §8
