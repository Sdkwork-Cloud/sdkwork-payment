import type { PaymentIntent } from './payment-intent';

export interface PaymentIntentResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
