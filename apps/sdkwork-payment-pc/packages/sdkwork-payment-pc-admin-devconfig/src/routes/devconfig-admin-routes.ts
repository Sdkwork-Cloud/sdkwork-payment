/**
 * Route contribution for the payment dev-config admin surface.
 *
 * Base path `/admin/payments/devconfig` aligns with the admin module registry in
 * `@sdkwork/payment-pc-admin-core` (createSdkworkPaymentPcAdminModuleRegistry) and
 * the OpenAPI contracts under `/backend/v3/api/payments/certificates`,
 * `/backend/v3/api/payments/webhook_events`, and `/backend/v3/api/payments/dev/*`.
 */

export const PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES = {
  moduleId: "payment-devconfig",
  basePath: "/admin/payments/devconfig",
  defaultPath: "/admin/payments/devconfig",
  permissionPrefix: "commerce.payments.devconfig",
} as const;

export type PaymentPcAdminDevConfigRouteManifest = typeof PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES;
