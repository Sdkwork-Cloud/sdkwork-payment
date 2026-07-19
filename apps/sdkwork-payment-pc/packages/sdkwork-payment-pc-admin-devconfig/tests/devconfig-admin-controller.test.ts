import { describe, expect, it, vi } from "vitest";
import type { SdkworkPaymentBackendService } from "@sdkwork/payment-service";

import { createPaymentDevConfigAdminController } from "../src/services/devconfig-admin-controller";

function listResult() {
  return {
    items: [],
    pageInfo: { hasMore: false, mode: "offset", page: 1, pageSize: 20, totalItems: "0" },
  };
}

function createBackendService() {
  const providerAccounts = vi.fn().mockResolvedValue(listResult());
  const certificates = vi.fn().mockResolvedValue(listResult());
  const webhookEvents = vi.fn().mockResolvedValue(listResult());
  const service = {
    certificates: { list: certificates },
    providerAccounts: { list: providerAccounts },
    webhookEvents: { list: webhookEvents },
  } as unknown as SdkworkPaymentBackendService;
  return { certificates, providerAccounts, service, webhookEvents };
}

describe("createPaymentDevConfigAdminController", () => {
  it("loads only provider resources for the independent environment page", async () => {
    const backend = createBackendService();
    const controller = createPaymentDevConfigAdminController({ service: backend.service });

    await controller.load("environment");

    expect(backend.providerAccounts).toHaveBeenCalledOnce();
    expect(backend.certificates).not.toHaveBeenCalled();
    expect(backend.webhookEvents).not.toHaveBeenCalled();
  });

  it("loads provider context and recent events for the webhook debugger", async () => {
    const backend = createBackendService();
    const controller = createPaymentDevConfigAdminController({ service: backend.service });

    await controller.load("webhook");

    expect(backend.providerAccounts).toHaveBeenCalledOnce();
    expect(backend.webhookEvents).toHaveBeenCalledOnce();
    expect(backend.certificates).not.toHaveBeenCalled();
  });
});
