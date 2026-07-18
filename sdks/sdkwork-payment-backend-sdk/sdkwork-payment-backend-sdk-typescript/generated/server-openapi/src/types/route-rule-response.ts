import type { RouteRule } from './route-rule';

export interface RouteRuleResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
