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

Queued webhook worker (`process_queued_webhook_events`) drains `commerce_payment_webhook_event` rows for admin replay and returns `payment_attempt_contexts` for the **order** host to settle.

**Forbidden:** Payment routes calling order HTTP, `settle_owner_order_after_payment_success`, or account adjustments.

## 5. Provider credentials

- Deployment defaults: env vars (`STRIPE_*`, `ALIPAY_*`, `WECHAT_PAY_*`, `PAYMENT_WEBHOOK_BASE_URL`).
- Tenant overrides: `commerce_payment_provider_account.secret_ref` stores env var **names** resolved at runtime for pay, close, refund, and webhook verify (Stripe/Alipay route via `out_trade_no` or Alipay `app_id`; WeChat Pay uses deployment env until `out_trade_no` routing is available pre-decrypt).

## 6. API prefixes

| Prefix | Role |
| --- | --- |
| `/app/v3/api/payments` | Methods, intents, records, statistics |
| `/backend/v3/api/payments` | Admin: providers, channels, webhooks, reconcile |

Deprecated `/app/v3/api/recharges/*` proxy is **opt-in only** (`SDKWORK_PAYMENT_ENABLE_RECHARGE_PROXY=1`). New clients must use order app-api. Local payment recharge handlers and repository SQL were removed in P3.

## 7. Migration status

| Location | Role | Status |
| --- | --- | --- |
| `recharge_proxy_router.rs` | Deprecated `/recharges` proxy → order | **Opt-in** (`SDKWORK_PAYMENT_ENABLE_RECHARGE_PROXY=1`) |
| ~~`sqlite_recharge.rs`~~ | Legacy order insert in payment | **Removed (P3)** |
| ~~`recharge_router.rs`~~ | Legacy local handlers | **Removed (P3)** |

## 8. Dependencies

| Direction | Allowed |
| --- | --- |
| Payment → Order (read order, validate payability) | Yes |
| Payment → Account | **No** (direct) |
| Order → Payment (create intent) | Yes |

## 9. SDK

- `@sdkwork/payment-app-sdk`: `payments.*`, `refunds.*` only
- No `recharges.orders.create` in target-state payment SDK ( lives on order SDK )

## 10. Verification

- Payment tests: intent requires valid orderId
- No test inserts `commerce_order` in payment crate (after migration)

Track phases in [commerce-boundary.spec.json](./commerce-boundary.spec.json).
