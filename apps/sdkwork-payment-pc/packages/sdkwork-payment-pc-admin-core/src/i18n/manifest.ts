import { createSdkworkMessageCatalog } from "@sdkwork/i18n-pc-react";
import { paymentAdminEnglishMessages } from "./en-US/commerce/payment-admin/legacy";
import { paymentAdminChineseMessages } from "./zh-CN/commerce/payment-admin/legacy";
import type { PaymentAdminMessages } from "./types";

export const PAYMENT_ADMIN_I18N_CATALOG = createSdkworkMessageCatalog<PaymentAdminMessages>({
  defaultLocale: "en-US",
  locales: { "en-US": paymentAdminEnglishMessages, "zh-CN": paymentAdminChineseMessages },
  namespace: "commerce.payment.admin",
});
