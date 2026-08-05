import type { PaymentIntent } from './payment-intent';

export interface PaymentIntentResponse {
  code: 0;
  data: unknown & { item: PaymentIntent; };
  /** Server-owned request correlation id. */
  traceId: string;
}
