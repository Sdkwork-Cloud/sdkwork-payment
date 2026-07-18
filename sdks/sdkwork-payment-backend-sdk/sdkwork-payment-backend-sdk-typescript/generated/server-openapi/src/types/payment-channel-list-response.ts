import type { PageInfo } from './page-info';
import type { PaymentChannel } from './payment-channel';

export interface PaymentChannelListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
