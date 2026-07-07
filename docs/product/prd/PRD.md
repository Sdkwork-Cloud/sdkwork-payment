# Payment PRD

Status: active
Owner: SDKWork maintainers
Application: payment
Updated: 2026-07-06
Specs: REQUIREMENTS_SPEC.md, DOCUMENTATION_SPEC.md

## Document Map

- Commerce repository dissolution: `../sdkwork-specs/MIGRATION_SPEC.md` §8

## 1. Background And Problem

Payments, intents, refunds, and provider admin require strict idempotency, auditability, and provider isolation.

This repository is a **T1 commerce capability building block**. The `sdkwork-commerce (deleted)` monolith has been dissolved; this repository is self-contained with its own domain logic, persistence, HTTP route builders, API server, and IAM middleware for the **payment** capability.

## 2. Target Users

Buyers, finance operators, payment integrators, and reconciliation staff.

## 3. Goals And Non-Goals

### Goals

- Own payment SQL, app payment/refund surfaces, and backend payment admin routes.
- Keep write operations protected by command headers and tenant-scoped stores.

### Non-Goals

- Order header lifecycle and points recharge checkout (owned by order capability).
- Raw provider HTTP without domain service boundaries.

## 4. Scope

- Payment methods, records, statistics, reconcile flows.
- Payment intents, attempts, and owner-order payment orchestration.
- Refunds.
- Backend payment admin: methods, providers, channels, route rules, webhooks, reconciliation.

Primary API prefixes:

- App: `/app/v3/api/payments`, `/app/v3/api/refunds`
- Backend: `/backend/v3/api/payments`

Migration status: **complete** (Phase 5 production hardening closed — see PRD §7).

## 5. User Scenarios

- A buyer pays for a pending order; payment record transitions to success with idempotent writes.
- An operator configures provider accounts and reviews webhook replay from backend admin routes.

## 6. Success Metrics

- Payment standard tests pass in payment service crate.
- Commerce standalone-gateway payment tests pass through IAM thin wrappers.

## 7. Phases

- Phase 1 (complete): payment SQL + app/backend routers migrated.
- Phase 2 (complete): payment_intent/refund SQL owned by payment repository.
- Phase 3 (complete): SDK contract route `/payments/attempts/{paymentAttemptId}` owned by payment app router.
- Phase 4 (complete): owner-order pay/cancel payment side-effects owned by owner-order payment stores.
- Phase 5 (complete): production hardening — command envelopes on all write routes, SQL pagination (including app payment methods), PSP enrichment with `callback_payload` persistence, checkout re-enrichment, DB-first close with best-effort PSP cancel, refund PSP submit with transient retry, webhook audit for unmatched events, envelope/trace alignment per `API_SPEC.md` / `PAGINATION_SPEC.md`.

## 8. Linked Requirements

- Commerce repository dissolution: `../sdkwork-specs/MIGRATION_SPEC.md` §8
- Component contract: `specs/component.spec.json`
- Machine contracts: local `specs/`, `database/ddl/`, route manifests

## 9. Open Questions

- Provider credential storage encryption policy and `PaymentProviderPort` implementation before external channel go-live.
- Dedicated async refund-retry worker deployment topology (queue consumer) for multi-instance gateways when inline PSP retries are insufficient.
