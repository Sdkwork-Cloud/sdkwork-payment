import { describe, expect, it, vi } from "vitest";
import type { SdkworkPaymentBackendService } from "@sdkwork/payment-service";

import { createPaymentMonitorAdminController } from "../src/services/monitor-admin-controller";

function createBackendService(intentsList: ReturnType<typeof vi.fn>): SdkworkPaymentBackendService {
  const emptyList = vi.fn().mockResolvedValue({
    items: [],
    pageInfo: { hasMore: false, mode: "cursor", totalItems: "0" },
  });
  return {
    attempts: { list: emptyList },
    intents: {
      list: intentsList,
      retrieve: vi.fn(),
    },
    reconciliationRuns: {
      create: vi.fn(),
      list: emptyList,
    },
    webhookEvents: {
      list: emptyList,
      replay: vi.fn(),
    },
  } as unknown as SdkworkPaymentBackendService;
}

describe("createPaymentMonitorAdminController", () => {
  it("refreshes payment records through the current server-side filter", async () => {
    const intentsList = vi.fn().mockResolvedValue({
      items: [{
        amount: "9.99",
        createdAt: "2026-07-19T00:00:00.000Z",
        currencyCode: "CNY",
        id: "intent-failed",
        orderId: "order-1",
        ownerUserId: "user-1",
        paymentIntentNo: "pay-failed",
        paymentMethod: "sandbox_test",
        providerCode: "sandbox",
        status: "failed",
        updatedAt: "2026-07-19T00:01:00.000Z",
      }],
      pageInfo: { hasMore: false, mode: "cursor", totalItems: "1" },
    });
    const controller = createPaymentMonitorAdminController({
      service: createBackendService(intentsList),
    });

    await controller.applyIntentFilter({ providerCode: "sandbox", status: "failed" });
    await controller.refreshIntents();

    expect(intentsList).toHaveBeenCalledTimes(2);
    expect(intentsList).toHaveBeenLastCalledWith(expect.objectContaining({
      page_size: 20,
      providerCode: "sandbox",
      status: "failed",
    }));
    expect(controller.getState().intents).toHaveLength(1);
    expect(controller.getState().status).toBe("ready");
  });
});
