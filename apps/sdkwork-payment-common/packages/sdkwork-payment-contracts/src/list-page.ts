/**
 * Server-backed list pagination helpers for sdkwork-payment admin/console surfaces.
 *
 * Mirrors `@sdkwork/iam-contracts/list-page.ts` to keep parity with PAGINATION_SPEC.md §2
 * (push filtering/sorting/paging to the server; no client-side slice over unbounded fetches).
 */

/** Default interactive list page size per `PAGINATION_SPEC.md` section 3. */
export const SDKWORK_DEFAULT_LIST_PAGE_SIZE = 20;

/** Maximum allowed list page size per `PAGINATION_SPEC.md` section 3. */
export const SDKWORK_MAX_LIST_PAGE_SIZE = 200;

function clampListPageSize(pageSize: number | undefined): number | undefined {
  if (pageSize === undefined) {
    return undefined;
  }
  return Math.min(Math.max(1, Math.floor(pageSize)), SDKWORK_MAX_LIST_PAGE_SIZE);
}

export type SdkWorkPageMode = "offset" | "cursor";

export interface SdkWorkPageInfo {
  readonly hasMore?: boolean;
  readonly mode?: SdkWorkPageMode;
  readonly nextCursor?: string | null;
  readonly page?: number;
  readonly pageSize?: number;
  readonly totalItems?: string;
  readonly totalPages?: number;
}

export interface SdkWorkListPage<T> {
  readonly items: readonly T[];
  readonly pageInfo?: SdkWorkPageInfo;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/**
 * Unwrap list items from SDK-unwrapped `SdkWorkPageData` or raw HTTP `data` payloads.
 */
export function extractSdkWorkListItems<T = unknown>(value: unknown): readonly T[] {
  if (Array.isArray(value)) {
    return value as readonly T[];
  }
  if (!isRecord(value)) {
    return [];
  }
  if (Array.isArray(value.items)) {
    return value.items as readonly T[];
  }
  if (isRecord(value.data) && Array.isArray(value.data.items)) {
    return value.data.items as readonly T[];
  }
  return [];
}

/**
 * Unwrap standard list pagination metadata alongside items.
 */
export function extractSdkWorkListPage<T = unknown>(value: unknown): SdkWorkListPage<T> {
  const items = extractSdkWorkListItems<T>(value);
  if (!isRecord(value)) {
    return { items };
  }
  const pageInfoSource = isRecord(value.pageInfo)
    ? value.pageInfo
    : isRecord(value.data) && isRecord(value.data.pageInfo)
      ? value.data.pageInfo
      : undefined;
  if (!pageInfoSource) {
    return { items };
  }
  return {
    items,
    pageInfo: {
      hasMore: typeof pageInfoSource.hasMore === "boolean" ? pageInfoSource.hasMore : undefined,
      mode: pageInfoSource.mode === "offset" || pageInfoSource.mode === "cursor" ? pageInfoSource.mode : undefined,
      nextCursor:
        typeof pageInfoSource.nextCursor === "string"
          ? pageInfoSource.nextCursor
          : pageInfoSource.nextCursor === null
            ? null
            : undefined,
      page: typeof pageInfoSource.page === "number" ? pageInfoSource.page : undefined,
      pageSize: typeof pageInfoSource.pageSize === "number" ? pageInfoSource.pageSize : undefined,
      totalItems: typeof pageInfoSource.totalItems === "string" ? pageInfoSource.totalItems : undefined,
      totalPages: typeof pageInfoSource.totalPages === "number" ? pageInfoSource.totalPages : undefined,
    },
  };
}

/**
 * Unwrap single resource item from SDK-unwrapped or raw HTTP `data` payloads.
 */
export function extractSdkWorkResourceItem<T = unknown>(value: unknown): T | undefined {
  if (!isRecord(value)) {
    return undefined;
  }
  if ("item" in value) {
    return value.item as T;
  }
  if (isRecord(value.data) && "item" in value.data) {
    return value.data.item as T;
  }
  return value as T;
}

const LIST_QUERY_KEYS = new Set(["page", "page_size", "pageSize", "cursor", "sort", "q"]);

function readOptionalString(value: unknown): string | undefined {
  if (typeof value !== "string") {
    return undefined;
  }
  const normalized = value.trim();
  return normalized || undefined;
}

function readOptionalNumber(value: unknown): number | undefined {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : undefined;
  }
  return undefined;
}

/** Build a standard offset/cursor list query. */
export function buildSdkWorkListQuery(input?: {
  readonly cursor?: string;
  readonly page?: number;
  readonly pageSize?: number;
  readonly q?: string;
  readonly sort?: string;
}): Record<string, string | number> {
  const query: Record<string, string | number> = {};
  const useCursor = Boolean(input?.cursor?.trim());
  if (!useCursor && input?.page !== undefined) {
    query.page = input.page;
  }
  if (input?.pageSize !== undefined) {
    query.page_size = clampListPageSize(input.pageSize) ?? SDKWORK_DEFAULT_LIST_PAGE_SIZE;
  }
  if (useCursor && input?.cursor) {
    query.cursor = input.cursor.trim();
  }
  if (input?.sort) {
    query.sort = input.sort;
  }
  if (input?.q) {
    query.q = input.q;
  }
  if (!("page_size" in query)) {
    query.page_size = SDKWORK_DEFAULT_LIST_PAGE_SIZE;
  }
  return query;
}

/**
 * Merge loose controller params with canonical list query defaults.
 * Preserves domain filters (for example `providerCode`) while enforcing `page_size`.
 */
export function resolveSdkWorkListQuery(
  params?: Record<string, unknown>,
): Record<string, string | number> {
  const query = buildSdkWorkListQuery({
    page: readOptionalNumber(params?.page),
    pageSize: clampListPageSize(
      readOptionalNumber(params?.page_size ?? params?.pageSize),
    ) ?? SDKWORK_DEFAULT_LIST_PAGE_SIZE,
    cursor: readOptionalString(params?.cursor),
    q: readOptionalString(params?.q),
    sort: readOptionalString(params?.sort),
  });

  if (!params) {
    return query;
  }

  for (const [key, value] of Object.entries(params)) {
    if (LIST_QUERY_KEYS.has(key) || value === undefined || value === null) {
      continue;
    }
    if (typeof value === "string" || typeof value === "number") {
      query[key] = value;
      continue;
    }
    if (typeof value === "boolean") {
      query[key] = String(value);
    }
  }

  return query;
}

/** Build the next server-backed list query from current `pageInfo`. */
export function buildNextSdkWorkListQuery(
  params: Record<string, unknown> | undefined,
  pageInfo: SdkWorkPageInfo | undefined,
): Record<string, string | number> | undefined {
  if (!pageInfo?.hasMore) {
    return undefined;
  }
  if (pageInfo.mode === "cursor") {
    if (!pageInfo.nextCursor) {
      return undefined;
    }
    const base = { ...params };
    delete base.page;
    delete base.pageNo;
    delete base.page_no;
    return resolveSdkWorkListQuery({ ...base, cursor: pageInfo.nextCursor });
  }
  const base = { ...params };
  delete base.cursor;
  const nextPage = (pageInfo.page ?? 1) + 1;
  return resolveSdkWorkListQuery({ ...base, page: nextPage });
}

/** Merge a newly fetched list page into controller state. */
export function mergeSdkWorkListPage<T>(
  current: readonly T[],
  page: SdkWorkListPage<T>,
  mode: "replace" | "append",
): SdkWorkListPage<T> {
  if (mode === "append") {
    return {
      items: [...current, ...page.items],
      pageInfo: page.pageInfo,
    };
  }
  return page;
}

export interface CreateSdkWorkPagedListSessionOptions<T> {
  fetchPage: (query: Record<string, string | number>) => Promise<unknown>;
  mapItem: (value: unknown) => T | undefined;
}

/** Mutable server-backed list session for admin/console controllers. */
export interface SdkWorkPagedListSession<T> {
  getItems(): readonly T[];
  getPageInfo(): SdkWorkPageInfo | undefined;
  list(params?: Record<string, unknown>): Promise<readonly T[]>;
  loadMore(params?: Record<string, unknown>): Promise<readonly T[]>;
  reset(): void;
  setItems(items: readonly T[]): void;
}

/**
 * Create a reusable paged-list session that tracks query params, items, and pageInfo.
 * Controllers use this to avoid duplicating offset/cursor append logic across surfaces.
 */
export function createSdkWorkPagedListSession<T>(
  options: CreateSdkWorkPagedListSessionOptions<T>,
): SdkWorkPagedListSession<T> {
  let lastListParams: Record<string, unknown> | undefined;
  let items: readonly T[] = [];
  let listPageInfo: SdkWorkPageInfo | undefined;

  const applyPage = (params: Record<string, unknown> | undefined, append: boolean): Promise<readonly T[]> => {
    if (!append) {
      lastListParams = params ? { ...params } : undefined;
    }
    const query = resolveSdkWorkListQuery(append ? lastListParams : params);
    return options.fetchPage(query).then((response) => {
      const page = extractSdkWorkListPage(response);
      const mapped = page.items
        .map(options.mapItem)
        .filter((item): item is T => item !== undefined);
      const merged = mergeSdkWorkListPage(append ? items : [], { ...page, items: mapped }, append ? "append" : "replace");
      items = merged.items;
      listPageInfo = merged.pageInfo;
      return items;
    });
  };

  return {
    getItems: () => items,
    getPageInfo: () => (listPageInfo ? { ...listPageInfo } : undefined),
    list: (params) => applyPage(params, false),
    loadMore: async (params) => {
      if (params) {
        lastListParams = { ...(lastListParams ?? {}), ...params };
      }
      const nextQuery = buildNextSdkWorkListQuery(lastListParams, listPageInfo);
      if (!nextQuery) {
        return items;
      }
      lastListParams = { ...nextQuery };
      return applyPage(lastListParams, true);
    },
    reset: () => {
      lastListParams = undefined;
      items = [];
      listPageInfo = undefined;
    },
    setItems: (next) => {
      items = [...next];
    },
  };
}
