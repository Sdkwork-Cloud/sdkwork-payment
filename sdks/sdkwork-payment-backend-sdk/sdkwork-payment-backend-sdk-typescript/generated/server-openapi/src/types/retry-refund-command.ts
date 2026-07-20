export interface RetryRefundCommand {
  /** Exact refund number typed by the operator. */
  confirmRefundNo: string;
  expectedStatus: 'failed';
}
