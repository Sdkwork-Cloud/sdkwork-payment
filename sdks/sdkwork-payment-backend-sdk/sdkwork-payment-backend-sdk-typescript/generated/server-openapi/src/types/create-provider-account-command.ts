export interface CreateProviderAccountCommand {
  accountNo: string;
  providerCode: 'stripe' | 'alipay' | 'wechat_pay' | 'sandbox';
  merchantId: string;
  accountMode?: 'direct' | 'partner';
  partnerProviderAccountId?: string;
  environment: 'development' | 'sandbox' | 'production';
  countryCode: string;
  settlementCurrency: string;
  /** Primary PSP secret material. Encrypted before database persistence and never returned. */
  primarySecret: string;
  /** Stripe webhook secret or WeChat API v3 key. */
  webhookSecret?: string;
  /** Alipay public key or WeChat platform certificate PEM. */
  certificate?: string;
  capabilities?: Record<string, unknown>;
  status?: 'active' | 'inactive' | 'suspended' | 'deprecated';
  metadata?: Record<string, unknown>;
}
