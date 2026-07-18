import { useSdkworkModuleMessages } from "@sdkwork/i18n-pc-react";
import { PAYMENT_ADMIN_I18N_CATALOG } from "./manifest";

export * from "./manifest";
export type * from "./messages";

export function usePaymentAdminMessages() {
  return useSdkworkModuleMessages(PAYMENT_ADMIN_I18N_CATALOG);
}
