import { describe, expect, it, vi } from "vitest";
import type { SdkworkPaymentBackendService } from "@sdkwork/payment-service";

import { createPaymentDevConfigAdminController } from "../src/services/devconfig-admin-controller";
import { PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES } from "../src/routes/devconfig-admin-routes";

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

describe("PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES", () => {
  it("publishes independently permissioned integration sections", () => {
    expect(
      Object.values(PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES.sections).map(
        (section) => section.path,
      ),
    ).toEqual([
      "/admin/payments/devconfig/environments",
      "/admin/payments/devconfig/webhook-debugger",
      "/admin/payments/devconfig/certificates",
      "/admin/payments/devconfig/logs",
    ]);
    expect(
      PAYMENT_PC_ADMIN_DEVCONFIG_ROUTES.sections.certificates
        .requiredPermissions,
    ).toEqual([
      "commerce.payments.certificates.read",
      "commerce.payments.certificates.create",
      "commerce.payments.certificates.delete",
    ]);
  });
});
