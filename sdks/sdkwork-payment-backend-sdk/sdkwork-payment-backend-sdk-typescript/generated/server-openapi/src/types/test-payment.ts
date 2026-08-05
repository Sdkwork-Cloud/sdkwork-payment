export interface TestPayment {
  paymentId: string;
  paymentIntentId: string;
  paymentIntentNo?: string;
  attemptId: string;
  outTradeNo: string;
  methodKey: string;
  providerCode: string;
  amount: string;
  currencyCode: string;
  /** Payment attempt status (pending, succeeded, failed, ...) */
  status: string;
  /** Scan-to-pay QR code payload (e.g., WeChat native code_url) when the provider returned one */
  qrCodeUrl?: string;
  expiresAt?: string;
  createdAt: string;
}
