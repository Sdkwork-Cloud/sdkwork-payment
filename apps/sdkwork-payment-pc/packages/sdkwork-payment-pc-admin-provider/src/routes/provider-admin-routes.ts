/**
 * Route contribution for the payment provider admin surface.
 *
 *basePath `/admin/payments/providers` aligns with the admin module registry in
 * `@sdkwork/payment-pc-admin-core` (createSdkworkPaymentPcAdminModuleRegistry) and
 * the OpenAPI contract under `/backend/v3/api/payments/provider_accounts`.
 */

export const PAYMENT_PC_ADMIN_PROVIDER_ROUTES = {
  moduleId: "payment-provider",
  basePath: "/admin/payments/providers",
  defaultPath: "/admin/payments/providers",
  permissionPrefix: "commerce.payments.provider_accounts",
} as const;

export type PaymentPcAdminProviderRouteManifest = typeof PAYMENT_PC_ADMIN_PROVIDER_ROUTES;
