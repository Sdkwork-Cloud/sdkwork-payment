export interface CreateTestPaymentCommand {
  /** Payment method key (e.g., wechat_native, alipay_qr, sandbox_test); must reference an active QR-code-capable method */
  methodKey: string;
  /** Test amount (defaults to 0.01) */
  amount?: string;
  /** Test currency (defaults to CNY) */
  currencyCode?: string;
}
