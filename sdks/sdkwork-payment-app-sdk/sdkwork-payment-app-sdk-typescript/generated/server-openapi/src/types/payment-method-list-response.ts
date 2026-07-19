import type { PageInfo } from './page-info';
import type { PaymentMethod } from './payment-method';

export interface PaymentMethodListResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
