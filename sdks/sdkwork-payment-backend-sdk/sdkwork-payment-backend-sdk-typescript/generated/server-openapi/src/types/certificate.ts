/** PEM certificate reference for provider credential management. */
export interface Certificate {
  id?: string;
  certificateNo?: string;
  providerCode?: 'stripe' | 'alipay' | 'wechat_pay';
  certificateType?: 'merchant_private_key' | 'provider_public_key' | 'platform_certificate' | 'webhook_secret';
  /** Env var name where the PEM/secret is stored */
  certificateRef?: string;
  /** SHA-256 fingerprint of the PEM for dedup/rotation tracking */
  fingerprint?: string;
  expiresAt?: string;
  issuer?: string;
  subject?: string;
  status?: 'active' | 'expired' | 'revoked' | 'pending_rotation';
  metadata?: Record<string, unknown>;
  createdAt?: string;
  updatedAt?: string;
}
