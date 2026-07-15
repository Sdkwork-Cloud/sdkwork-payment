/**
 * Payment admin monitor route manifest.
 *
 * `moduleId` aligns with the `id` registered in
 * `createSdkworkPaymentPcAdminModuleRegistry()` (admin-core), and
 * `basePath` / `permissionPrefix` mirror the same record. Route resolution
 * is owned by the host app; this constant is the single source of truth for
 * navigation metadata exposed by this capability package.
 */

export const PAYMENT_PC_ADMIN_MONITOR_ROUTES = {
  moduleId: "payment-monitor",
  basePath: "/admin/payments/monitor",
  defaultPath: "/admin/payments/monitor",
  permissionPrefix: "commerce.payments.intents",
} as const;

export type PaymentPcAdminMonitorRouteManifest = typeof PAYMENT_PC_ADMIN_MONITOR_ROUTES;
