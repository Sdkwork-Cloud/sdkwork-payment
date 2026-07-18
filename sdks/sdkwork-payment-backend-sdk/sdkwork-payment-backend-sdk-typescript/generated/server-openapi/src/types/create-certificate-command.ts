export interface CreateCertificateCommand {
  certificateNo: string;
  providerCode: 'stripe' | 'alipay' | 'wechat_pay';
  certificateType: 'merchant_private_key' | 'provider_public_key' | 'platform_certificate' | 'webhook_secret';
  /** Env var name; the PEM content is read from env at runtime, never stored in DB */
  certificateRef: string;
  /** Optional base64 PEM for parsing expiry/fingerprint server-side; not persisted */
  pemContent?: string;
  metadata?: Record<string, unknown>;
}
