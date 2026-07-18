export interface ProviderAccount {
  id?: string;
  accountNo?: string;
  providerCode?: 'stripe' | 'alipay' | 'wechat_pay' | 'sandbox';
  merchantId?: string;
  /** direct = merchant self-connection; partner = ISV/service provider mode with sub-merchants */
  accountMode?: 'direct' | 'partner';
  /** Parent partner account id when accountMode=partner; null when direct */
  partnerProviderAccountId?: string;
  environment?: 'development' | 'sandbox' | 'production';
  countryCode?: string;
  settlementCurrency?: string;
  /** Env var name for primary secret (Stripe secret key, Alipay/WeChat private key PEM). Never stores plaintext. */
  secretRef?: string;
  /** Env var name for webhook secret (Stripe) or WeChat API v3 key */
  webhookSecretRef?: string;
  /** Env var name for Alipay public key or WeChat platform cert PEM */
  certificateRef?: string;
  /** Supported payment capabilities: { pay: true, refund: true, close: true, query: true } */
  capabilities?: Record<string, unknown>;
  status?: 'active' | 'inactive' | 'suspended' | 'deprecated';
  /** Provider-specific extras: appId, merchantSerialNo, returnUrl, sub_appid mappings */
  metadata?: Record<string, unknown>;
  /** Parsed PEM expiry for certificate_ref */
  certificateExpiresAt?: string;
  lastTestedAt?: string;
  lastTestStatus?: 'success' | 'failure' | 'unknown';
  createdAt?: string;
  updatedAt?: string;
}
