# Payment Executor Spec

Status: active
Owner: SDKWork maintainers
Capability: `commerce.payment`
Updated: 2026-07-26

Authority: Payment PRD Non-Goals, `sdkwork-specs/API_SPEC.md`

## 1. Purpose

`sdkwork-payment` executes provider payment collection and provider refunds against an existing `orderId`. It does not own commerce order headers, recharge package catalog, account ledger effects, refund business approval, or withdrawal business approval.

Provider payout is a future executor boundary owned by payment. It is not a current payment API and must stay fail-closed in order until a concrete provider payout executor contract is published.

## 2. Single Responsibility

| Owns | Does not own |
| --- | --- |
| `commerce_payment_intent` | `commerce_order` insert/update lifecycle |
| `commerce_payment_attempt` | `commerce_recharge_package` |
| Provider accounts and channels | Recharge order creation |
| Refund records and provider refund calls | Account ledger |
| Future provider payout executor contract boundary | Withdrawal request approval or account hold settlement |
| Reconciliation runs | Unified order list UI |
| Webhook event persistence through a port | PSP webhook HTTP routes owned by order |

## 3. Required Inputs

All write operations must reference order-owned evidence:

- `payments.intents.create` -> `orderId` required.
- `refunds.create` -> `orderId` required.
- Owner-order pay side effects -> via order `orders.payments.create` orchestration.
- Future provider payout execution -> via an order-owned withdrawal request and a future payment executor contract.

## 4. Webhook Port

PSP notify URLs target `sdkwork-order`:

```text
POST /app/v3/api/orders/payments/webhooks/{providerCode}
```

Payment exposes repository ports only:

```text
Order webhook handler
  -> verify/normalize through payment-providers
  -> ingest_provider_webhook through payment repository
  -> order in-process settlement saga
```

Legacy `POST /app/v3/api/payments/webhooks/{providerCode}` returns `410 Gone`.

Backend admin `webhook_events` replay re-applies stored payment attempt status only. It does not run order fulfillment. Use order `payment_confirmations` for settlement recovery.

Forbidden: payment routes calling order HTTP, `settle_owner_order_after_payment_success`, or account adjustments.

## 5. Provider Credentials

- Deployment compatibility defaults may still read legacy `STRIPE_*`, `ALIPAY_*`, and `WECHAT_PAY_*` variables; new provider-account configuration never requires them. `ORDER_PAYMENT_WEBHOOK_BASE_URL` remains non-secret deployment routing configuration.
- Notify URL pattern: `{base}/app/v3/api/orders/payments/webhooks/{providerCode}`; order gateway owns HTTP.
- Tenant provider credentials are write-only backend inputs encrypted into `commerce_payment_provider_credential`. Runtime resolution is database-first for pay, close, refund, and webhook verification; existing `*_ref` account columns are compatibility-only fallbacks and are never returned by APIs.
- Stripe and Alipay route through `out_trade_no` or Alipay `app_id`.
- WeChat Pay uses deployment env until `out_trade_no` routing is available before decrypt.
- Bootstrap profiles pre-wire provider methods/channels in tenant default organization `0` and keep production provider accounts inactive. Runtime resolution uses the authenticated organization first, then organization `0`, then legacy unscoped rows; an organization-specific method/account overrides inherited defaults. Runtime method eligibility requires an active account (or an explicitly unbound channel using deployment-level credentials); duplicate active accounts at the winning organization scope fail closed.
- The production bootstrap includes a mock WeChat account and `006_upgrade_bootstrap_templates.sql` repairs only untouched legacy mock rows. Replace identifiers and secret references, pass the provider-account dry-run, and activate the account to expose the pre-wired methods.
- Provider-account activation is a status-only operation after configuration is saved. The latest dry-run must be successful and at least as recent as the saved configuration, and bootstrap mock markers must be removed. New accounts default to `inactive`.
- Runtime routing selects an eligible active channel for the requested method and currency within the authenticated organization, inherited organization `0`, or a legacy unscoped row. Matching active route rules win by rule priority, followed by scene match, channel priority, sort order, and stable channel id. Method listing applies the same scope and account eligibility and selects at most one effective row per `method_key`; it must not expose a global method by borrowing a channel from another organization. A configured but unavailable channel set fails closed across all organization scopes; deployment credentials are only a compatibility fallback when the method has never had a channel.
- The selected `commerce_payment_channel.id` and its `providerAccountId` are persisted as one immutable Attempt routing snapshot (`channel_id` plus `callback_payload`). Checkout, close, and refund use the snapshotted account rather than the channel's current binding or an arbitrary active account by provider code. Every historical PSP operation verifies that the account provider matches the persisted Attempt provider and fails closed on drift. Legacy attempts without the account snapshot may resolve their historical channel once for compatibility.
- An inactive or deprecated account may service close/refund for its historical attempts; a suspended, deleted, missing, or cross-tenant account fails closed. New payments always require an active bound account.
- The current order-owned webhook URL is provider-scoped, so one tenant/organization may have only one active account per provider at a time. Activation rejects a second active account. Merchant rotation is sequential: deactivate the old account, validate and activate the new account; historical attempts remain bound to the old account for close/refund.

## 6. API Prefixes

| Prefix | Role |
| --- | --- |
| `/app/v3/api/payments` | Methods, intents, records, statistics |
| `/app/v3/api/refunds` | Refund create/list/retrieve |
| `/backend/v3/api/payments` | Admin: providers, channels, webhooks, reconcile |

Points recharge (`/app/v3/api/recharges/*`) is owned by `sdkwork-order` only. Payment must not expose recharge HTTP routes, proxy recharge calls, or publish recharge service contract operations.

## 7. Dependencies

| Direction | Allowed |
| --- | --- |
| Payment -> Order crate/package dependency | No |
| Payment -> `commerce_order` read-only SQL validation | Yes, repository-local snapshots only |
| Payment -> Account | No |
| Order -> Payment in-process ports, including `OwnerOrderPaymentStore` | Yes |

Payment must not depend on `sdkwork-order` Rust crates. Order lifecycle types consumed across capabilities, such as `PayOwnerOrderCommand` and `OwnerOrderPaymentConfirmationPort`, are defined in `sdkwork-payment-service` and re-exported by `sdkwork-order-service` for order routers only.

Machine contract: [commerce-dependency-boundary.spec.json](./commerce-dependency-boundary.spec.json).

## 8. SDK

- `@sdkwork/payment-app-sdk`: `payments.*` and `refunds.*` only.
- Recharge (`recharges.*`) lives on the order SDK (`@sdkwork/order-app-sdk` or an approved composed order surface).
- Withdrawal request creation lives on the order SDK (`withdrawals.requests.*`). Payment must not expose wallet withdrawal request APIs.

## 9. Write Response Envelopes

Create operations return HTTP `201` with the created resource under `data.item`. Update operations
return HTTP `200` with the updated resource under `data.item`, and deletes return HTTP `204`
without a JSON body. Domain commands such as cancel, close, replay, test, and rotate return an
HTTP `200` `SdkWorkApiResponse` command payload:

```json
{
  "code": 0,
  "data": {
    "accepted": true,
    "resourceId": "optional-resource-id",
    "status": "optional-status"
  },
  "traceId": "server-trace-id"
}
```

Read routes return `data.item` or `data.items` plus `data.pageInfo`.

App reconcile (`POST /payments/reconcile`) is a lookup command that returns the latest payment record as `data.item`. It performs no inline PSP status repair.

## 10. PSP Enrichment And Persistence

After the repository persists intent/attempt, `enrich_owner_order_payment_*` calls the configured PSP and merges `providerTransactionId` / `providerStatus` into attempt `callback_payload` for later close/cancel.

`GET /payments/checkout/{paymentId}` re-enriches pending attempts for cashier and QR parameters.

Payer inputs needed for pending checkout recreation (`openid`, `client_ip`) are stored under the `paymentMetadata` namespace in `callback_payload`; repository persistence merges that namespace with any domain callback fields instead of replacing them, and legacy flat callback payloads remain readable. PSP enrichment fields stay outside that namespace.

The persisted payment attempt is the immutable PSP request authority. Initial checkout and every replay use its original `provider_code`, `channel_id`, `providerAccountId`, `out_trade_no`, business `idempotency_key`, `paymentScene`, and `paymentMetadata`; a later click's command idempotency key, channel configuration, scene, or payer metadata must not mutate an existing provider request. Every one-step and two-step Attempt insert persists its `request_no` and `idempotency_key`. Provider HTTP idempotency keys are fixed-width ASCII SHA-256 derivatives of that immutable snapshot rather than raw client input. Same-method clicks reuse one PSP trade across different HTTP idempotency keys only when the persisted scene and payer-metadata snapshots also match. A different scene or payer snapshot creates a separate Attempt, while reusing one business idempotency key with changed payment method, payer metadata, or payment scene returns conflict.

Close invokes the PSP with the attempt's historical channel/account before committing the local closed state. PSP failure leaves the local payment retryable. If a concurrent webhook settles the attempt while provider close is in flight, settlement wins: the local close returns conflict and replacement checkout must stop instead of creating a second trade. A replay against an absent or already canceled/closed provider trade remains idempotent. Provider cancel and refund commands use deterministic operation-scoped idempotency keys. Refund submission and reconciliation use the same historical account so activating another merchant cannot redirect funds; Stripe uses the persisted `pi_*` provider transaction id, while Alipay and WeChat use the persisted merchant trade number. A provider response explicitly reporting terminal success transitions the refund to `succeeded`; an explicit terminal provider failure transitions it to `failed`; unknown, accepted, pending, and ambiguous network/provider outcomes remain `processing` and continue reserving refundable amount. A processing replay queries the PSP by the original payment identity and merchant refund number, persists a reported terminal state, and submits with the same refund identity only when the PSP conclusively confirms that no matching refund exists. Query transport, authentication, malformed/truncated response, or ambiguous provider failures leave the refund in `processing`. Replays reuse the same refund number and provider idempotency key. Deterministic local submission validation/context failures also transition to `failed`, including both canonical `validation` and compatibility `validation-failed` error codes.

Payment reads `commerce_order.expired_at` as part of its repository-local Order reference. A payable status is insufficient by itself: a missing, empty, expired, or malformed expiry fails closed. Each new `commerce_payment_attempt` initially copies the Order expiry; provider enrichment narrows it to `min(order expiry, creation time + 15 minutes)`, sends WeChat `time_expire` or Alipay `timeout_express`, and exposes the same timestamp as payment param `expiresAt`. Alipay's `timeout_express` is minute-granular, so an invalid or expired timestamp and any remaining lifetime below 60 seconds fail closed; the adapter must never round a sub-minute Order boundary up to one minute. Pending attempts are reusable only while both the attempt and Order have explicit future expiry timestamps; a stale QR therefore creates a fresh attempt under the same Order only when the Order itself is still payable. Before provider enrichment, the current attempt is revalidated by tenant, organization, Order, status, and deletion state; a superseded waiter returns conflict without calling a provider. Expired-attempt cleanup performed by checkout is restricted to that locked Order, then conflicting attempts for the same Order are closed through their historical account. The revalidate, close, provider checkout, and provider-result persistence sequence is serialized by tenant, organization, and Order: SQLite standalone runtimes use a process-local keyed async mutex, while PostgreSQL uses a bounded transaction-scoped advisory-lock retry that does not retain connections for waiting requests and requires a pool capacity of at least two. PSP failure leaves local state retryable. Provider responses that explicitly identify an absent or terminally closed trade (`ORDER_NOT_EXIST`, WeChat `ORDER_CLOSED`, `ACQ.TRADE_NOT_EXIST`, Stripe `resource_missing`, or Stripe's already-canceled state) are treated as idempotent close success; other provider errors remain failures. Provider enrichment atomically merges PSP identity/status into the callback snapshot and fails closed if the active Attempt disappeared. New provider trade numbers are deterministic 32-character ASCII SHA-256 derivatives, compatible with the strictest supported PSP limit, and identity remains unique by `(tenant_id, provider_code, out_trade_no)` for non-deleted attempts.

## 11. Verification

- Payment tests: intent requires a valid `orderId`.
- No production code inserts `commerce_order` in payment crates.
- No payment crate or package depends on account.
- No payment app-api exposes recharge or wallet withdrawal request routes.

Track phases in [commerce-boundary.spec.json](./commerce-boundary.spec.json). Webhook boundary: [commerce-payment-webhook.spec.json](./commerce-payment-webhook.spec.json).
