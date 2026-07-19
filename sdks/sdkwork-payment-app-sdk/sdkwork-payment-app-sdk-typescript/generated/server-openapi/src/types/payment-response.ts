import type { Payment } from './payment';

export interface PaymentResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
