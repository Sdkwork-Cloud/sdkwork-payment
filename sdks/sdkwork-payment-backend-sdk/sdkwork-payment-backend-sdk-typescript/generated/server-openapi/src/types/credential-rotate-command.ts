export interface CredentialRotateCommand {
  /** New env var name for primary secret */
  secretRef: string;
  /** New env var name for webhook secret */
  webhookSecretRef?: string;
  /** New env var name for certificate PEM */
  certificateRef?: string;
  /** Mark previous refs as deprecated (metadata flag) */
  invalidatePrevious?: boolean;
}
