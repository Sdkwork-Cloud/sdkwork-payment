import type { PageInfo } from './page-info';
import type { PaymentMethod } from './payment-method';

export interface PaymentMethodListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
