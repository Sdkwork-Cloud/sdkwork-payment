import type { PageInfo } from './page-info';
import type { PaymentIntent } from './payment-intent';

export interface PaymentIntentListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
