# sdkwork-api-payment-assembly Specs

Component root: `crates/sdkwork-api-payment-assembly`

Gateway assembly manifest, business-router composition, and verification contract.

The standalone App API contribution retains the deprecated Payment webhook `410` shim. Hosts that
also compose the Order App API use `assemble_federated_app_api_contribution_from_env`; its Router
and manifest both contain only active Payment and Refund operations.
