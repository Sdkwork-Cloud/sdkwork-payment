import type { PageInfo } from './page-info';
import type { ReconciliationRun } from './reconciliation-run';

export interface ReconciliationRunListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
