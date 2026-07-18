import type { PageInfo } from './page-info';
import type { WebhookEvent } from './webhook-event';

export interface WebhookEventListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
