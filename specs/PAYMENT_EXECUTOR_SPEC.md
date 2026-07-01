# Payment Executor Spec

Status: active  
Owner: SDKWork maintainers  
Capability: `commerce.payment`  
Updated: 2026-06-29

Authority: Payment PRD Non-Goals, `sdkwork-specs/API_SPEC.md`

## 1. Purpose

**sdkwork-payment** executes payment collection and refunds against an **existing** `orderId`. It does not own commerce order headers or recharge package catalog.

## 2. Single responsibility

| Owns | Does not own |
| --- | --- |
| `commerce_payment_intent` | `commerce_order` insert/update lifecycle |
| `commerce_payment_attempt` | `commerce_recharge_package` |
| Provider accounts, channels, webhooks | Recharge order creation |
| Refund records & provider refund calls | Account ledger |
| Reconciliation runs | Unified order list UI |

## 3. Required inputs

All write operations **must** reference Order:

- `payments.intents.create` → `orderId` required
- `refunds.create` → `orderId` required
- Owner-order pay side-effects → via Order `orders.pay` orchestration

## 4. Webhook & saga

```text
Provider webhook → Payment updates attempt
                → notify Order (payment_status)
                → Order saga → Account adjustments
```

**Forbidden:** Payment webhook → direct Account backend adjustment.

## 5. API prefixes

| Prefix | Role |
| --- | --- |
| `/app/v3/api/payments` | Methods, intents, records, statistics |
| `/backend/v3/api/payments` | Admin: providers, channels, webhooks, reconcile |

Target: **remove** `/app/v3/api/recharges/*` proxy after one release (clients must use order). Local payment recharge handlers and repository SQL were removed in P3.

## 6. Migration status

| Location | Role | Status |
| --- | --- | --- |
| `recharge_proxy_router.rs` | Deprecated `/recharges` proxy → order | **Active** — remove after proxy window |
| ~~`sqlite_recharge.rs`~~ | Legacy order insert in payment | **Removed (P3)** |
| ~~`recharge_router.rs`~~ | Legacy local handlers | **Removed (P3)** |

## 7. Dependencies

| Direction | Allowed |
| --- | --- |
| Payment → Order (read order, validate payability) | Yes |
| Payment → Account | **No** (direct) |
| Order → Payment (create intent) | Yes |

## 8. SDK

- `@sdkwork/payment-app-sdk`: `payments.*`, `refunds.*` only
- No `recharges.orders.create` in target-state payment SDK ( lives on order SDK )

## 9. Verification

- Payment tests: intent requires valid orderId
- No test inserts `commerce_order` in payment crate (after migration)

Track phases in [commerce-boundary.spec.json](./commerce-boundary.spec.json).
