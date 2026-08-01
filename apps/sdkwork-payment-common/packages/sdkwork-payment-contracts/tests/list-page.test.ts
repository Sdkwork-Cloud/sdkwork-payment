import { describe, expect, it } from "vitest";

import {
  buildSdkWorkListQuery,
  createSdkWorkPagedListSession,
  extractSdkWorkResourceItem,
  resolveSdkWorkListQuery,
} from "../src/list-page";

describe("extractSdkWorkResourceItem", () => {
  it("accepts the resource object returned by an unwrapped generated SDK", () => {
    const resource = { id: "payment-1", status: "pending" };

    expect(extractSdkWorkResourceItem(resource)).toBe(resource);
  });

  it("accepts item and full-envelope resource payloads", () => {
    const resource = { id: "payment-2" };

    expect(extractSdkWorkResourceItem({ item: resource })).toBe(resource);
    expect(extractSdkWorkResourceItem({ data: { item: resource } })).toBe(resource);
  });

  it("rejects non-object payloads", () => {
    expect(extractSdkWorkResourceItem(undefined)).toBeUndefined();
    expect(extractSdkWorkResourceItem("payment-3")).toBeUndefined();
  });
});

describe("generated SDK list parameters", () => {
  it("uses pageSize in TypeScript and leaves page_size serialization to the generated client", () => {
    expect(buildSdkWorkListQuery({ page: 2, pageSize: 40 })).toEqual({
      page: 2,
      pageSize: 40,
    });
  });

  it("rejects HTTP wire and legacy aliases at the SDK service boundary", () => {
    for (const key of ["page_size", "limit", "page_no", "pageNo", "per_page", "size"]) {
      expect(() => resolveSdkWorkListQuery({ [key]: 20 })).toThrow(/use pageSize/);
    }
  });

  it("passes bounded pageSize parameters to generated SDK list methods", async () => {
    const received: Record<string, unknown>[] = [];
    const session = createSdkWorkPagedListSession<{ id: string }>({
      fetchPage: async (query) => {
        received.push(query);
        return {
          items: [{ id: "provider-1" }],
          pageInfo: { mode: "offset", page: 1, pageSize: 20, hasMore: false },
        };
      },
      mapItem: (value) => value as { id: string },
    });

    await session.list();

    expect(received).toEqual([{ pageSize: 20 }]);
    expect(session.getPageInfo()).toEqual({
      mode: "offset",
      page: 1,
      pageSize: 20,
      hasMore: false,
      nextCursor: undefined,
      totalItems: undefined,
      totalPages: undefined,
    });
  });

  it("ignores an older list response after a newer query has completed", async () => {
    const resolvers = new Map<string, (value: unknown) => void>();
    const session = createSdkWorkPagedListSession<{ id: string }>({
      fetchPage: (query) => new Promise((resolve) => {
        resolvers.set(String(query.q), resolve);
      }),
      mapItem: (value) => value as { id: string },
    });

    const oldRequest = session.list({ q: "old" });
    const newRequest = session.list({ q: "new" });
    resolvers.get("new")?.({
      items: [{ id: "new-result" }],
      pageInfo: { mode: "offset", page: 1, pageSize: 20, hasMore: false },
    });
    await newRequest;
    resolvers.get("old")?.({
      items: [{ id: "stale-result" }],
      pageInfo: { mode: "offset", page: 1, pageSize: 20, hasMore: false },
    });
    await oldRequest;

    expect(session.getItems()).toEqual([{ id: "new-result" }]);
  });

  it("coalesces concurrent load-more calls so one page is appended once", async () => {
    let fetchCount = 0;
    let resolveNextPage: ((value: unknown) => void) | undefined;
    const session = createSdkWorkPagedListSession<{ id: string }>({
      fetchPage: async () => {
        fetchCount += 1;
        if (fetchCount === 1) {
          return {
            items: [{ id: "page-1" }],
            pageInfo: { mode: "offset", page: 1, pageSize: 20, hasMore: true },
          };
        }
        return new Promise((resolve) => {
          resolveNextPage = resolve;
        });
      },
      mapItem: (value) => value as { id: string },
    });
    await session.list();

    const first = session.loadMore();
    const second = session.loadMore();
    expect(second).toBe(first);
    expect(fetchCount).toBe(2);
    resolveNextPage?.({
      items: [{ id: "page-2" }],
      pageInfo: { mode: "offset", page: 2, pageSize: 20, hasMore: false },
    });

    await Promise.all([first, second]);
    expect(session.getItems()).toEqual([{ id: "page-1" }, { id: "page-2" }]);
  });
});
