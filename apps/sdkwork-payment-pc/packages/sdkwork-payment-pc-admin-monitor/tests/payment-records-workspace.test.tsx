import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  assertSdkworkCatalogLocaleParity,
  SdkworkI18nProvider,
} from "@sdkwork/i18n-pc-react";
import { PAYMENT_ADMIN_I18N_CATALOG } from "@sdkwork/payment-pc-admin-core";

import { IntentMonitor } from "../src/components/IntentMonitor";
import { PAYMENT_RECORDS_I18N_CATALOG } from "../src/i18n";
import { PaymentMonitorAdminWorkspace } from "../src/pages/MonitorAdminWorkspace";
import type {
  PaymentIntentDetail,
  PaymentIntentView,
  PaymentMonitorAdminController,
  PaymentMonitorAdminState,
} from "../src/types/monitor-admin-types";

afterEach(() => {
  cleanup();
});

const intents: readonly PaymentIntentView[] = [
  {
    amount: "128.50",
    createdAt: "2026-07-19T01:00:00.000Z",
    currencyCode: "CNY",
    id: "intent-1",
    orderId: "order-1001",
    ownerUserId: "user-2001",
    paymentIntentNo: "pay_20260719_0001",
    paymentMethod: "stripe_card",
    providerCode: "stripe",
    status: "succeeded",
    updatedAt: "2026-07-19T01:01:00.000Z",
  },
  {
    amount: "42.00",
    createdAt: "2026-07-19T02:00:00.000Z",
    currencyCode: "CNY",
    id: "intent-2",
    orderId: "order-1002",
    ownerUserId: "user-2002",
    paymentIntentNo: "pay_20260719_0002",
    paymentMethod: "alipay_pc",
    providerCode: "alipay",
    status: "failed",
    updatedAt: "2026-07-19T02:02:00.000Z",
  },
  {
    amount: "10.00",
    createdAt: "2026-07-19T03:00:00.000Z",
    currencyCode: "CNY",
    id: "intent-3",
    orderId: "order-1003",
    ownerUserId: "user-2003",
    paymentIntentNo: "pay_20260719_0003",
    paymentMethod: "wechat_native",
    providerCode: "wechat_pay",
    status: "succeeded",
    updatedAt: "2026-07-19T03:01:00.000Z",
  },
];

const selectedIntent: PaymentIntentDetail = {
  ...intents[0]!,
  attempts: [{
    amount: "128.50",
    attemptNo: "attempt-001",
    channelId: "channel-stripe-card",
    createdAt: "2026-07-19T01:00:10.000Z",
    currencyCode: "CNY",
    id: "attempt-1",
    paymentIntentId: "intent-1",
    providerCode: "stripe",
    providerTransactionId: "pi_123456",
    status: "succeeded",
  }],
  metadata: { source: "manager-test" },
};

function renderMonitor(overrides: Partial<React.ComponentProps<typeof IntentMonitor>> = {}) {
  const props: React.ComponentProps<typeof IntentMonitor> = {
    intents,
    onApplyFilter: vi.fn(),
    onLoadMore: vi.fn(),
    onRefresh: vi.fn(),
    onSelect: vi.fn(),
    pageInfo: { hasMore: true, mode: "cursor", nextCursor: "next-1", totalItems: "42" },
    ...overrides,
  };
  return { props, ...render(<IntentMonitor {...props} />) };
}

function createIdleController(): PaymentMonitorAdminController {
  const state: PaymentMonitorAdminState = {
    attempts: [],
    intents: [],
    reconciliationRuns: [],
    status: "ready",
    webhookEvents: [],
  };
  return {
    applyAttemptFilter: async () => [],
    applyIntentFilter: async () => [],
    applyReconciliationRunFilter: async () => [],
    applyWebhookEventFilter: async () => [],
    createReconciliationRun: async () => {
      throw new Error("Not used by this test.");
    },
    getState: () => state,
    load: async () => state,
    loadMoreAttempts: async () => [],
    loadMoreIntents: async () => [],
    loadMoreReconciliationRuns: async () => [],
    loadMoreWebhookEvents: async () => [],
    refreshIntents: async () => [],
    replayWebhookEvent: async (eventId) => ({
      eventId,
      ok: true,
      replayedAt: "2026-07-19T00:00:00.000Z",
    }),
    selectIntent: async () => undefined,
    subscribe: () => () => undefined,
  };
}

describe("payment records workspace", () => {
  it("keeps English and Chinese payment-record catalogs in parity", () => {
    expect(() => assertSdkworkCatalogLocaleParity(PAYMENT_RECORDS_I18N_CATALOG)).not.toThrow();
    expect(PAYMENT_RECORDS_I18N_CATALOG.resolveMessages("zh-CN").table.title).toBe("支付记录");
    expect(PAYMENT_RECORDS_I18N_CATALOG.resolveMessages("en-US").table.title).toBe("Payment records");
  });

  it("does not run the legacy token translator over the localized payment-record workspace", async () => {
    const { container } = render(
      <SdkworkI18nProvider
        catalogs={[PAYMENT_ADMIN_I18N_CATALOG, PAYMENT_RECORDS_I18N_CATALOG]}
        locale="zh-CN"
      >
        <PaymentMonitorAdminWorkspace
          capabilities={{ canCreateReconciliationRun: false, canReplayWebhookEvent: false }}
          controller={createIdleController()}
        />
      </SdkworkI18nProvider>,
    );

    await waitFor(() => expect(screen.getByText("成功支付金额")).toBeInTheDocument());
    expect(screen.getByText("支付筛选")).toBeInTheDocument();
    expect(screen.getByText("支付对账")).toBeInTheDocument();
    expect(container.textContent).not.toMatch(/paymentsFilters|succeededpayments|Viewpayments/u);
  });

  it("renders a route-bound operational section without duplicate tab navigation", async () => {
    const { container } = render(
      <SdkworkI18nProvider
        catalogs={[PAYMENT_ADMIN_I18N_CATALOG, PAYMENT_RECORDS_I18N_CATALOG]}
        locale="zh-CN"
      >
        <PaymentMonitorAdminWorkspace
          capabilities={{ canCreateReconciliationRun: false, canReplayWebhookEvent: false }}
          controller={createIdleController()}
          description="审计支付机构回调。"
          section="webhooks"
          title="Webhook 事件"
        />
      </SdkworkI18nProvider>,
    );

    await waitFor(() => {
      expect(container.querySelector('[data-slot="payment-webhook-event-monitor"]')).not.toBeNull();
    });
    expect(screen.getByRole("heading", { name: "Webhook 事件" })).toBeInTheDocument();
    expect(screen.getByText("事件类型")).toBeInTheDocument();
    expect(screen.getByText(/暂无 Webhook 事件/u)).toBeInTheDocument();
    expect(screen.queryByRole("tab")).not.toBeInTheDocument();
    expect(container.querySelector('[data-slot="payment-intent-monitor"]')).toBeNull();
    expect(container.textContent).not.toMatch(/No webhook|Apply filter|Clear filters/u);
  });

  it("renders server-result metrics and a dense payment records table", () => {
    renderMonitor();

    expect(screen.getByText("42")).toBeInTheDocument();
    expect(screen.getByText("66.7%")).toBeInTheDocument();
    expect(screen.getByText("2/3")).toBeInTheDocument();
    expect(screen.getByText("Showing 3 of 42")).toBeInTheDocument();
    expect(screen.getByText("pay_20260719_0001")).toBeInTheDocument();
    expect(screen.getByText("order-1001")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Refresh payment records" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /view payment details: pay_20260719_0001/i })).toBeInTheDocument();
  });

  it("applies advanced business filters and preserves an explicit reset", async () => {
    const onApplyFilter = vi.fn().mockResolvedValue(undefined);
    renderMonitor({ onApplyFilter });

    fireEvent.click(screen.getByRole("button", { name: "Advanced filters" }));
    fireEvent.change(screen.getByLabelText("Order ID"), { target: { value: "order-1002" } });
    fireEvent.change(screen.getByLabelText("Currency"), { target: { value: "cny" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply filters" }));

    await waitFor(() => {
      expect(onApplyFilter).toHaveBeenCalledWith(expect.objectContaining({
        currencyCode: "CNY",
        orderId: "order-1002",
      }));
    });

    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
    await waitFor(() => expect(onApplyFilter).toHaveBeenLastCalledWith({}));
  });

  it("refreshes through the injected controller action and opens auditable payment detail", async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    const onSelect = vi.fn().mockResolvedValue(undefined);
    renderMonitor({ onRefresh, onSelect, selectedIntent });

    fireEvent.click(screen.getByRole("button", { name: "Refresh payment records" }));
    await waitFor(() => expect(onRefresh).toHaveBeenCalledTimes(1));

    fireEvent.click(screen.getByRole("button", { name: /view payment details: pay_20260719_0001/i }));
    await waitFor(() => expect(onSelect).toHaveBeenCalledWith(intents[0]));
    expect(screen.getByText("Payment timeline")).toBeInTheDocument();
    expect(screen.getByText(/pi_123456/)).toBeInTheDocument();
    expect(screen.getByText(/manager-test/)).toBeInTheDocument();
  });
});
