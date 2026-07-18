export interface ProviderAccountTestCommand {
  /** Override target environment for this test (defaults to account.environment) */
  environment?: 'development' | 'sandbox' | 'production';
  /** Validate env var resolution without invoking PSP API */
  dryRun?: boolean;
}
