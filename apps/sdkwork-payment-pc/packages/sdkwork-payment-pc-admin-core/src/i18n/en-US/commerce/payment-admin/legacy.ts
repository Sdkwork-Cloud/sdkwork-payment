import type { PaymentAdminMessages } from "../../../types";
import { paymentAdminChineseMessages } from "../../../zh-CN/commerce/payment-admin/legacy";

export const paymentAdminEnglishMessages: PaymentAdminMessages = {
  legacy: {
    phrases: Object.fromEntries(
      Object.keys(paymentAdminChineseMessages.legacy.phrases).map((key) => [key, key]),
    ),
    tokens: Object.fromEntries(
      Object.keys(paymentAdminChineseMessages.legacy.tokens).map((key) => [key, key]),
    ),
  },
};
