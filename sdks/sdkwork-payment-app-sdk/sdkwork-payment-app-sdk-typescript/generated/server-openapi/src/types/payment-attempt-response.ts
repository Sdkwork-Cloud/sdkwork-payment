import type { PaymentAttempt } from './payment-attempt';

export interface PaymentAttemptResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
