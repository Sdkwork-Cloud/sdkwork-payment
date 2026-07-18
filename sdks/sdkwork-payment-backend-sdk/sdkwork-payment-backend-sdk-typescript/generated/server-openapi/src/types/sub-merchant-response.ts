import type { SubMerchant } from './sub-merchant';

export interface SubMerchantResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
