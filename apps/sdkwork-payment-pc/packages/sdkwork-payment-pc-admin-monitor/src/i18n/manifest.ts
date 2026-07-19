import { createSdkworkMessageCatalog } from "@sdkwork/i18n-pc-react";

import { paymentRecordsWorkspaceMessages as enWorkspace } from "./en-US/commerce/payment-records/workspace";
import { paymentRecordsWorkspaceMessages as zhWorkspace } from "./zh-CN/commerce/payment-records/workspace";
import type { PaymentRecordsMessages } from "../types/payment-records-i18n";

export const PAYMENT_RECORDS_I18N_CATALOG = createSdkworkMessageCatalog<PaymentRecordsMessages>({
  defaultLocale: "en-US",
  locales: {
    "en-US": enWorkspace,
    "zh-CN": zhWorkspace,
  },
  namespace: "commerce.payment.paymentRecords",
});
