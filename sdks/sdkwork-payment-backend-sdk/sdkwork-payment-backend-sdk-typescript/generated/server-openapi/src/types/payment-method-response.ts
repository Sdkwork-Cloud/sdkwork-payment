import type { PaymentMethod } from './payment-method';

export interface PaymentMethodResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
