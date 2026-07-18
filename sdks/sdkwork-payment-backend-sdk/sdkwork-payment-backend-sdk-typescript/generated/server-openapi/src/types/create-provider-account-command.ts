export interface CreateProviderAccountCommand {
  accountNo: string;
  providerCode: 'stripe' | 'alipay' | 'wechat_pay' | 'sandbox';
  merchantId: string;
  accountMode?: 'direct' | 'partner';
  partnerProviderAccountId?: string;
  environment: 'development' | 'sandbox' | 'production';
  countryCode: string;
  settlementCurrency: string;
  secretRef: string;
  webhookSecretRef?: string;
  certificateRef?: string;
  capabilities?: Record<string, unknown>;
  status?: 'active' | 'inactive' | 'suspended' | 'deprecated';
  metadata?: Record<string, unknown>;
}
