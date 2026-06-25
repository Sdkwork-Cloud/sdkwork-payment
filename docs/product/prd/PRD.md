# Payment PRD

Status: active
Owner: SDKWork maintainers
Application: payment
Updated: 2026-06-24
Specs: REQUIREMENTS_SPEC.md, DOCUMENTATION_SPEC.md

## Document Map

- Platform split alignment (commerce T0): `../sdkwork-commerce/docs/architecture/tech/TECH-2026-06-24-commerce-capability-repo-split-alignment.md`

## 1. Background And Problem

Payments, intents, refunds, recharge checkout, and provider admin require strict idempotency, auditability, and provider isolation.

This repository is a **T1 commerce capability building block**. `sdkwork-commerce` remains the T0 composition layer (gateway, IAM wrappers, composed SDK). This repository owns domain logic, persistence, and HTTP route builders for the **payment** capability.

## 2. Target Users

Buyers, finance operators, payment integrators, and reconciliation staff.

## 3. Goals And Non-Goals

### Goals

- Own payment/recharge SQL, app payment surfaces, and backend payment admin routes.
- Keep write operations protected by command headers and tenant-scoped stores.

### Non-Goals

- Order header lifecycle (owned by order capability).
- Raw provider HTTP without domain service boundaries.

## 4. Scope

- Payment methods, records, statistics, reconcile flows.
- Payment intents, attempts, and owner-order payment orchestration.
- Refunds.
- Points recharge checkout.
- Backend payment admin: methods, providers, channels, route rules, webhooks, reconciliation.

Primary API prefixes:

- App: `/app/v3/api/payments`
- Backend: `/backend/v3/api/payments`

Migration status: **complete**.

## 5. User Scenarios

- A buyer pays for a pending order; payment record transitions to success with idempotent writes.
- An operator configures provider accounts and reviews webhook replay from backend admin routes.
- A user purchases points through recharge checkout and polls checkout status.

## 6. Success Metrics

- Payment standard tests pass in payment service crate.
- Commerce api-server payment tests pass through IAM thin wrappers.

## 7. Phases

- Phase 1 (complete): payment/recharge SQL + app/backend routers migrated.
- Phase 2 (complete): payment_intent/refund SQL owned by payment repository.
- Phase 3 (complete): SDK contract route `/payments/attempts/{paymentAttemptId}` owned by payment app router.
- Phase 4 (complete): owner-order pay/cancel payment side-effects owned by `sqlite_owner_order_payment` / `postgres_owner_order_payment`.

## 8. Linked Requirements

- Commerce capability split alignment: `../sdkwork-commerce/docs/architecture/tech/TECH-2026-06-24-commerce-capability-repo-split-alignment.md`
- Component contract: `specs/component.spec.json` (when present)
- Machine contracts: local `specs/`, future `apis/`, and generated `sdks/`

## 9. Open Questions

- Provider credential storage encryption policy before production launch.
