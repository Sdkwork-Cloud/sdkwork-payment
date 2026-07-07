# sdkwork-payment Component Specs

Local specification index for the payment executor capability.

## Spec map

| Document | Purpose |
| --- | --- |
| [component.spec.json](./component.spec.json) | Workspace manifest |
| [PAYMENT_EXECUTOR_SPEC.md](./PAYMENT_EXECUTOR_SPEC.md) | Payment-only scope; no order header ownership |
| [commerce-boundary.spec.json](./commerce-boundary.spec.json) | Machine-readable boundaries + migration |
| [commerce-dependency-boundary.spec.json](./commerce-dependency-boundary.spec.json) | Payment ⊥ order crate dependency rule |
| [commerce-payment-webhook.spec.json](./commerce-payment-webhook.spec.json) | PSP webhook HTTP owned by order; payment ports only |

## Sibling specs

| Repository | Entry |
| --- | --- |
| `sdkwork-order` | `specs/RECHARGE_ORDER_SPEC.md`, `specs/commerce-payment-webhook.spec.json` (HTTP owner) |
| `sdkwork-account` | `specs/COMMERCE_BOUNDARY_SPEC.md` |

## Verification

```powershell
cd ..\sdkwork-payment
pnpm verify
cargo test --workspace
```
