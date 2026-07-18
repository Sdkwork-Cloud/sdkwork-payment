import { createSdkworkMessageCatalog } from "@sdkwork/i18n-pc-react";
import { paymentAdminChineseMessages, paymentAdminEnglishMessages, type PaymentAdminMessages } from "./messages";

export const PAYMENT_ADMIN_I18N_CATALOG = createSdkworkMessageCatalog<PaymentAdminMessages>({
  defaultLocale: "en-US",
  locales: { "en-US": paymentAdminEnglishMessages, "zh-CN": paymentAdminChineseMessages },
  namespace: "commerce.payment.admin",
});
