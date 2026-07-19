import type { PageInfo } from './page-info';
import type { Refund } from './refund';

export interface RefundListResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
