import type { Refund } from './refund';

export interface RefundResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
