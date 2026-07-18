import { describe, expect, it } from "vitest";

import { extractSdkWorkResourceItem } from "../src/list-page";

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
