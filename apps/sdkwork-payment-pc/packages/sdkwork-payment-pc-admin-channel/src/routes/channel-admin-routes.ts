/**
 * Route contribution for the payment channel admin surface.
 *
 * basePath `/admin/payments/channels` aligns with the admin module registry in
 * `@sdkwork/payment-pc-admin-core` (createSdkworkPaymentPcAdminModuleRegistry) and
 * the OpenAPI contracts under:
 *   - `/backend/v3/api/payments/methods`
 *   - `/backend/v3/api/payments/channels`
 *   - `/backend/v3/api/payments/route_rules`
 */

export const PAYMENT_PC_ADMIN_CHANNEL_ROUTES = {
  moduleId: "payment-channel",
  basePath: "/admin/payments/channels",
  defaultPath: "/admin/payments/channels",
  permissionPrefix: "commerce.payments.channels",
} as const;

export type PaymentPcAdminChannelRouteManifest = typeof PAYMENT_PC_ADMIN_CHANNEL_ROUTES;
