import type { PaymentChannel } from './payment-channel';

export interface PaymentChannelResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
