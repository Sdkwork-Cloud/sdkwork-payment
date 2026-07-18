import type { PageInfo } from './page-info';
import type { SubMerchant } from './sub-merchant';

export interface SubMerchantListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
