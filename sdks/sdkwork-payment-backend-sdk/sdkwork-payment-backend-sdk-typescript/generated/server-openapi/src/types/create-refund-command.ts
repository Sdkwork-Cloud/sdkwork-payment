export interface CreateRefundCommand {
  paymentIntentId: string;
  amount?: string;
  reasonCode: 'customer_request' | 'duplicate' | 'fraud' | 'service_failure' | 'other';
  /** Exact payment intent number typed by the operator as a high-risk action confirmation. */
  confirmPaymentIntentNo: string;
}
