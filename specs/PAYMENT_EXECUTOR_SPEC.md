# Payment Executor Spec

Status: active  
Owner: SDKWork maintainers  
Capability: `commerce.payment`  
Updated: 2026-07-06

Authority: Payment PRD Non-Goals, `sdkwork-specs/API_SPEC.md`

## 1. Purpose

**sdkwork-payment** executes payment collection and refunds against an **existing** `orderId`. It does not own commerce order headers or recharge package catalog.

## 2. Single responsibility

| Owns | Does not own |
| --- | --- |
| `commerce_payment_intent` | `commerce_order` insert/update lifecycle |
| `commerce_payment_attempt` | `commerce_recharge_package` |
| Provider accounts, channels | Recharge order creation |
| Refund records & provider refund calls | Account ledger |
| Reconciliation runs | Unified order list UI |
| Webhook event persistence (via port) | PSP webhook HTTP routes (owned by order) |

## 3. Required inputs

All write operations **must** reference Order:

- `payments.intents.create` → `orderId` required
- `refunds.create` → `orderId` required
- Owner-order pay side-effects → via Order `orders.pay` orchestration

## 4. Webhook port (order-owned HTTP)

PSP notify URLs target **sdkwork-order** (`POST /app/v3/api/orders/payments/webhooks/{providerCode}`). Payment exposes **repository ports only**:

```text
Order webhook handler → verify/normalize (payment-providers lib)
                     → ingest_provider_webhook (payment repository)
                     → order in-process settlement saga
```

Legacy `POST /app/v3/api/payments/webhooks/{providerCode}` returns **410 Gone**.

Backend admin `webhook_events` replay re-applies stored payment attempt status only (not order fulfillment). Use order `payment_confirmations` for settlement recovery.

**Forbidden:** Payment routes calling order HTTP, `settle_owner_order_after_payment_success`, or account adjustments.

## 5. Provider credentials

- Deployment defaults: env vars (`STRIPE_*`, `ALIPAY_*`, `WECHAT_PAY_*`, `ORDER_PAYMENT_WEBHOOK_BASE_URL`).
- Notify URL pattern: `{base}/app/v3/api/orders/payments/webhooks/{providerCode}` (order gateway owns HTTP).
- Tenant overrides: `commerce_payment_provider_account.secret_ref` stores env var **names** resolved at runtime for pay, close, refund, and webhook verify (Stripe/Alipay route via `out_trade_no` or Alipay `app_id`; WeChat Pay uses deployment env until `out_trade_no` routing is available pre-decrypt).

## 6. API prefixes

| Prefix | Role |
| --- | --- |
| `/app/v3/api/payments` | Methods, intents, records, statistics |
| `/app/v3/api/refunds` | Refund create/list/retrieve |
| `/backend/v3/api/payments` | Admin: providers, channels, webhooks, reconcile |

Points recharge (`/app/v3/api/recharges/*`) is owned by **sdkwork-order** only. Payment must not expose recharge HTTP routes, proxy, or service contract operations.

## 7. Dependencies

| Direction | Allowed |
| --- | --- |
| Payment → Order (crate / package dependency) | **No** |
| Payment → `commerce_order` (read-only SQL FK validation) | Yes (repository-local snapshots only) |
| Payment → Account | **No** (direct) |
| Order → Payment (in-process ports, `OwnerOrderPaymentStore`) | Yes |

Payment **must not** depend on `sdkwork-order` Rust crates. Order lifecycle types consumed across capabilities (`PayOwnerOrderCommand`, `OwnerOrderPaymentConfirmationPort`, …) are defined in `sdkwork-payment-service` and re-exported by `sdkwork-order-service` for order routers only.

Machine contract: [commerce-dependency-boundary.spec.json](./commerce-dependency-boundary.spec.json).

## 8. SDK

- `@sdkwork/payment-app-sdk`: `payments.*`, `refunds.*` only
- Recharge (`recharges.*`) lives on the order SDK (`@sdkwork/order-app-sdk` or commerce order surface)

## 10. Write response envelopes

All payment/refund **mutating** app-api and backend-api routes return `SdkWorkApiResponse` command payloads (`data.accepted`, optional `resourceId` / `status`) per `API_SPEC.md` §16. Read routes return `data.item` or `data.items` + `pageInfo`.

App reconcile (`POST /payments/reconciliations`) is a lookup command that returns the latest payment record as `data.item` (no PSP status repair inline).

## 11. PSP enrichment and persistence

After repository persists intent/attempt, `enrich_owner_order_payment_*` calls the configured PSP and merges `providerTransactionId` / `providerStatus` into attempt `callback_payload` for later close/cancel. `GET /payments/checkout/{paymentId}` re-enriches pending attempts for cashier/QR parameters.

## 12. Verification

- Payment tests: intent requires valid orderId
- No test inserts `commerce_order` in payment crate (after migration)

Track phases in [commerce-boundary.spec.json](./commerce-boundary.spec.json). Webhook boundary: [commerce-payment-webhook.spec.json](./commerce-payment-webhook.spec.json).
