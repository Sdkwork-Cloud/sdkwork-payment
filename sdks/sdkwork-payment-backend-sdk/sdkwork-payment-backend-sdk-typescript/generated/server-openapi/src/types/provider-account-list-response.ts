import type { PageInfo } from './page-info';
import type { ProviderAccount } from './provider-account';

export interface ProviderAccountListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
