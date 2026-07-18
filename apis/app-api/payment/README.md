# Payment App API

SDKWork-owned app/client contract for the `commerce.payment` capability.

- Authority OpenAPI: `sdkwork-payment-app-api.openapi.yaml`
- API prefixes: `/app/v3/api/payments`, `/app/v3/api/refunds`
- SDK family: `sdkwork-payment-app-sdk` -> `@sdkwork/payment-app-sdk`
- Auth: `dual-token`
- Success: `SdkWorkApiResponse` with numeric `code: 0`, `data`, and `traceId`
- Errors: `application/problem+json` with numeric `ProblemDetail.code` and `traceId`
- Lists: server-side offset pagination through `page` and `page_size`

The legacy payment webhook shim is runtime-only migration infrastructure. The active provider
webhook authority is owned by `sdkwork-order` at
`POST /app/v3/api/orders/payments/webhooks/{providerCode}`.

