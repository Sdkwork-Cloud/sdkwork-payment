/**
 * Admin module registry subpath.
 *
 * Exposes the module registry factory and record type for host apps that
 * need to enumerate registered admin capabilities. Aligns with
 * `CORE_EXPORT_SUBPATHS` in `sdkwork-specs/tools/lib/app-composition.mjs`.
 */

export {
  createSdkworkPaymentPcAdminModuleRegistry,
  type PaymentPcAdminModuleRecord,
} from "../index";
