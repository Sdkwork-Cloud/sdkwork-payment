import { ADMIN_PROVIDER_LABEL } from "@sdkwork/payment-pc-admin-core";
import type { PaymentStatus } from "../types/monitor-admin-types";

export const PAYMENT_STATUS_BADGE_VARIANT: Record<
  PaymentStatus,
  "default" | "success" | "warning" | "danger" | "secondary"
> = {
  canceled: "secondary",
  closed: "secondary",
  created: "default",
  failed: "danger",
  pending: "warning",
  processing: "warning",
  refunded: "success",
  refunding: "warning",
  succeeded: "success",
};

export function formatPaymentProvider(providerCode: string): string {
  if (providerCode in ADMIN_PROVIDER_LABEL) {
    return ADMIN_PROVIDER_LABEL[providerCode as keyof typeof ADMIN_PROVIDER_LABEL];
  }
  return providerCode || "--";
}
