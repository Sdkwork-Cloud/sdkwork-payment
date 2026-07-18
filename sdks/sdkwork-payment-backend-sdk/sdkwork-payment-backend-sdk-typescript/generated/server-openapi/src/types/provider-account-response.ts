import type { ProviderAccount } from './provider-account';

export interface ProviderAccountResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
