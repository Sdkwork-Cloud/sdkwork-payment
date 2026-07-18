import type { ProviderAccountTestResult } from './provider-account-test-result';

export interface ProviderAccountTestResultResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
