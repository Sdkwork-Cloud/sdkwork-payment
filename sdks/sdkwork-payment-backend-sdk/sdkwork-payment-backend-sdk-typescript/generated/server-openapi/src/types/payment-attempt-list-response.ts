import type { PageInfo } from './page-info';
import type { PaymentAttempt } from './payment-attempt';

export interface PaymentAttemptListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
