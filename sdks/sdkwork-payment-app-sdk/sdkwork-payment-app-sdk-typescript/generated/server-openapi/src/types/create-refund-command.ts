export interface CreateRefundCommand {
  orderId: string;
  paymentAttemptId?: string;
  amount?: string;
  reasonCode?: string;
}
