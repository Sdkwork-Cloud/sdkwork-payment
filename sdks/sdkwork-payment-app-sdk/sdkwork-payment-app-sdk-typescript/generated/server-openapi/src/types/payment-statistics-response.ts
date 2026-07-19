import type { PaymentStatistics } from './payment-statistics';

export interface PaymentStatisticsResponse {
  code: 0;
  data: Record<string, unknown>;
  traceId: string;
}
