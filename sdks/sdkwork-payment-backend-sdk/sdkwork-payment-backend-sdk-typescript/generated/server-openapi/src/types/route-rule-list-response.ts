import type { PageInfo } from './page-info';
import type { RouteRule } from './route-rule';

export interface RouteRuleListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
