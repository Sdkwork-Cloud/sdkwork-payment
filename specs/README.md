# sdkwork-payment Component Specs

Local specification index for the payment executor capability.

## Spec map

| Document | Purpose |
| --- | --- |
| [component.spec.json](./component.spec.json) | Workspace manifest |
| [PAYMENT_EXECUTOR_SPEC.md](./PAYMENT_EXECUTOR_SPEC.md) | Payment-only scope; no order header ownership |
| [commerce-boundary.spec.json](./commerce-boundary.spec.json) | Machine-readable boundaries + migration |

## Sibling specs

| Repository | Entry |
| --- | --- |
| `sdkwork-order` | `specs/RECHARGE_ORDER_SPEC.md` |
| `sdkwork-account` | `specs/COMMERCE_BOUNDARY_SPEC.md` |

## Verification

```powershell
cd ..\sdkwork-payment
pnpm verify
cargo test --workspace
```
