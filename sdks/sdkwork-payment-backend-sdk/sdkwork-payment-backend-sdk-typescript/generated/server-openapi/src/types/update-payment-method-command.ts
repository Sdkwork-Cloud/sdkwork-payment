export interface UpdatePaymentMethodCommand {
  displayName?: string;
  providerCode?: 'stripe' | 'alipay' | 'wechat_pay' | 'sandbox';
  status?: 'active' | 'inactive' | 'deprecated';
  currencyCode?: string;
  countryCode?: string;
  sortOrder?: number;
  metadata?: Record<string, unknown>;
}
