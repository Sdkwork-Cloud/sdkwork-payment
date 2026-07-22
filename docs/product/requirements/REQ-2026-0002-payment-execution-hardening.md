# REQ-2026-0002 Payment Execution Hardening

id: REQ-2026-0002
title: Harden payment execution correctness and reduce accidental complexity
owner: SDKWork payment maintainers
status: in-progress
source: reliability
problem: Payment execution has strong provider, channel, webhook, refund, and reconciliation boundaries, but idempotency replays do not consistently verify request identity, buyer refunds can bind an unscoped payment attempt, webhook signature verification does not enforce delivery freshness, and payment method naming has drifted between generic checkout methods and provider products.

## Goals

- Reject an idempotency key replay when the persisted payment or refund parameters differ from the new command.
- Scope every owner idempotency lookup by tenant, organization, and owner before returning payment data.
- Bind a refund only to a succeeded payment attempt from the same tenant, organization, owner, order, and currency.
- Reject stale Stripe and WeChat Pay webhook signatures using the provider-recommended five-minute tolerance.
- Preserve the existing order -> payment -> account ownership direction and historical channel/account binding.
- Record a reviewable simplification path for payment method naming and route-layer persistence.

## Non-Goals

- Changing public HTTP paths, generated SDKs, or response envelopes.
- Adding or modifying a database migration in this requirement.
- Moving order lifecycle, recharge packages, or the account ledger into payment.
- Replacing local credential encryption with a production KMS in this change.
- Completing the route/repository extraction or payment-method contract migration without human review.

## Acceptance Criteria

- Replaying a payment-intent idempotency key with another payment method returns a conflict.
- Owner idempotency lookup cannot return another owner or organization payment record.
- Replaying a refund idempotency key with a different amount, currency, reason, requester type, or explicit payment attempt returns a conflict.
- A supplied refund payment attempt must be succeeded and match tenant, organization, owner, order, and currency.
- Stripe and WeChat Pay signatures outside a 300-second clock-skew window fail verification.
- Exact webhook event deduplication, state-transition validation, and historical account routing remain unchanged.
- Root workspace membership is deterministic and contains no duplicate crate entry.
- Public contracts and generated SDK output remain unchanged.

## Non-Functional Requirements

- Security: fail closed on ambiguous scope, stale webhook signatures, unavailable credentials, and mismatched idempotency input.
- Privacy: do not expose cross-owner or cross-organization payment data during idempotency replay.
- Performance: new replay checks use existing indexed identity predicates and do not add unbounded queries.
- Reliability: PostgreSQL and SQLite implementations retain behavioral parity.

## Affected Surfaces

- backend
- persistence
- provider adapters
- architecture

## Trace

### Specs

- `REQUIREMENTS_SPEC.md`
- `ARCHITECTURE_DECISION_SPEC.md`
- `APPLICATION_LAYERED_ARCHITECTURE_SPEC.md`
- `DATABASE_SPEC.md`
- `SECURITY_SPEC.md`
- `INTEGRATION_SPEC.md`
- `RUST_CODE_SPEC.md`
- `TEST_SPEC.md`

### Components

- `crates/sdkwork-payment-service`
- `crates/sdkwork-payment-repository-sqlx`
- `crates/sdkwork-payment-providers`

## Verification

- `cargo test -p sdkwork-payment-providers`
- `cargo test -p sdkwork-payment-repository-sqlx`
- `cargo test -p sdkwork-payment-service`
- `cargo fmt --all -- --check`
- `node ../sdkwork-specs/tools/check-application-layering.mjs --root .`
- `node ../sdkwork-specs/tools/check-rust-backend-composition.mjs --root .`

## Human Review Gates

- Approve the proposed payment-method family/product contract before a public API or data migration.
- Approve a partial unique active-provider-account database constraint and rollout plan before migration.
- Approve production KMS requirements and fail-closed startup policy before changing credential runtime behavior.
- Approve extraction of SQL and provider orchestration from route crates before broad module movement.
