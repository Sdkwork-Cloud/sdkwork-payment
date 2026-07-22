# ADR-20260721 Payment Execution Hardening And Simplification

Status: proposed
Requirement: REQ-2026-0002
Owner: SDKWork payment maintainers
Date: 2026-07-21
Specs: ARCHITECTURE_DECISION_SPEC.md, APPLICATION_LAYERED_ARCHITECTURE_SPEC.md, DATABASE_SPEC.md, SECURITY_SPEC.md, INTEGRATION_SPEC.md

## Context

The payment bounded context currently covers several capabilities that are normal for a production payment executor: payment intents and attempts, provider accounts and credentials, channel routing, webhook ingestion, refunds, and reconciliation. Those capabilities are necessary, but implementation evidence shows accidental complexity and correctness gaps around them:

- application checkout uses a generic method such as `wechat_pay`, while provider adapters and catalogs also use product keys such as `wechat_native` and `wechat_jsapi`;
- route crates contain substantial SQL persistence and provider orchestration even though route crates should adapt HTTP to service ports;
- idempotency is enforced by database uniqueness but request identity is not consistently validated on replay;
- active provider-account uniqueness is enforced by read/check logic rather than an atomic database invariant;
- the standalone credential cipher uses a host-local wrapping key, which is unsuitable as an implicit multi-replica production default.

The repository has nine Rust crates and about thirty thousand lines of authored Rust. The number of deployable/runtime boundaries is reasonable, but the service layer is much smaller than the repository and route layers, indicating misplaced responsibilities rather than too many payment concepts.

## Decision

1. Keep the current commerce boundaries: order owns business orders and settlement orchestration, payment owns PSP execution and refunds, and account owns the ledger.
2. Keep provider adapters behind `PaymentProviderAdapter`, and keep provider account/channel selection fail closed.
3. Treat idempotency as both identity uniqueness and request-parameter consistency. A replay with different authoritative parameters is a conflict.
4. Treat webhook verification as signature, freshness, event deduplication, exact attempt identity, and monotonic state transition. A valid but stale Stripe or WeChat Pay signature is not sufficient.
5. Require refunds to resolve their original succeeded payment attempt from tenant, organization, owner, order, and currency evidence.
6. Preserve generic checkout method compatibility for current callers. A future reviewed contract will separate a customer-facing payment method family from the provider product/scene selected by routing; seeds are compatibility data, not the permanent naming model.
7. Simplify implementation in phases by moving SQL stores and provider orchestration out of route crates into repository/service modules. Do not collapse provider, persistence, route, assembly, or gateway boundaries merely to reduce crate count.
8. Defer database uniqueness migration and KMS startup policy to separately reviewed migration/security work.

## Alternatives

- Collapse payment into order: rejected because PSP credentials, webhook verification, reconciliation, and provider lifecycle are independent security and operational responsibilities.
- Split every table into another microservice: rejected because it increases distributed transaction and operational cost without improving the current bounded context.
- Rename `wechat_pay` immediately to product-specific keys: rejected because it would break existing order and client contracts.
- Accept idempotency keys without parameter checks: rejected because silent replay of a different financial command is unsafe and diverges from established PSP behavior.
- Rely only on event-id deduplication for webhook replay defense: rejected because signed-delivery freshness is an independent provider security control.

## Consequences

- Existing public APIs and SDKs remain compatible.
- Some previously accepted stale webhook deliveries or mismatched idempotency replays now fail closed.
- SQLite and PostgreSQL repositories need explicit parity tests.
- The route crates remain larger than the target architecture until the phased extraction is reviewed and completed.
- Production multi-replica credential encryption still requires an explicitly installed shared key provider or KMS implementation.

## Verification

- Provider unit tests cover fresh, future-skewed, and stale signatures.
- Repository integration tests cover replay conflicts, owner scope, and exact refund-attempt binding.
- Layering and Rust composition validators identify remaining route-to-SQL debt.
- API and SDK checks prove no public contract drift.

## Supersedes / Superseded By

None.
