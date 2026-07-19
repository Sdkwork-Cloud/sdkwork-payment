import { useSdkworkModuleMessages } from "@sdkwork/i18n-pc-react";

import { PAYMENT_RECORDS_I18N_CATALOG } from "./manifest";

export * from "./manifest";

export function usePaymentRecordsMessages() {
  return useSdkworkModuleMessages(PAYMENT_RECORDS_I18N_CATALOG);
}
