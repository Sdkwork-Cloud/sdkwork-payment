/**
 * SDK inventory subpath.
 *
 * Exposes the backend SDK inventory and capability package manifest for
 * composition tooling. Aligns with `CORE_EXPORT_SUBPATHS` in
 * `sdkwork-specs/tools/lib/app-composition.mjs`.
 */

export {
  PAYMENT_PC_ADMIN_CAPABILITY_PACKAGES,
  listSdkworkPaymentPcAdminSdkInventory,
  type PaymentPcAdminCapabilityPackage,
} from "../index";
