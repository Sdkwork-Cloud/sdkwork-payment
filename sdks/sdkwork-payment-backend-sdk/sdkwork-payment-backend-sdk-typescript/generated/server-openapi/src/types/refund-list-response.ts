import type { PageInfo } from './page-info';
import type { Refund } from './refund';

export interface RefundListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
