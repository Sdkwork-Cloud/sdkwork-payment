import type { Certificate } from './certificate';
import type { PageInfo } from './page-info';

export interface CertificateListResponse {
  code: 0;
  data: unknown & Record<string, unknown>;
  /** Server-owned request correlation id. */
  traceId: string;
}
