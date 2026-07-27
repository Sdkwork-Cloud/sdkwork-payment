# SDKWork Payment Providers Component Specs

This directory defines the local component contract for `sdkwork-payment-providers`.

- Component root: `sdkwork-payment/crates/sdkwork-payment-providers`
- Machine contract: `specs/component.spec.json`
- Public boundary: provider adapters and payment-provider operations exported by the crate root
- Security boundary: provider credentials remain in provider/runtime composition and are never exposed to app consumers
- Verification: `cargo test -p sdkwork-payment-providers`

Root standards under `../../../sdkwork-specs/` remain authoritative. This component does not own Orders, payment persistence, HTTP routes, or generated SDKs.
