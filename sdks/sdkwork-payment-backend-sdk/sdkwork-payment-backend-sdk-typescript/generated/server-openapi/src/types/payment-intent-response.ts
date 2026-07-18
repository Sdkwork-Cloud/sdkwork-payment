import type { PaymentIntent } from './payment-intent';

export interface PaymentIntentResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
