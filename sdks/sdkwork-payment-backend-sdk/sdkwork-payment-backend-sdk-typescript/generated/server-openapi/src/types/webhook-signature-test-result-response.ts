import type { WebhookSignatureTestResult } from './webhook-signature-test-result';

export interface WebhookSignatureTestResultResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
