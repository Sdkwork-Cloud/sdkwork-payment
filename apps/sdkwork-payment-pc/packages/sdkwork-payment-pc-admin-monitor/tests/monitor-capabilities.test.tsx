import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import { ReconciliationMonitor } from "../src/components/ReconciliationMonitor";
import { WebhookEventMonitor } from "../src/components/WebhookEventMonitor";

describe("payment monitor capabilities", () => {
  it("does not render webhook replay controls for read-only operators", () => {
    render(
      <WebhookEventMonitor
        canReplay={false}
        events={[{
          id: "event-1",
          eventId: "evt-1",
          eventType: "payment.succeeded",
          providerCode: "stripe",
          status: "failed",
          retries: 0,
          receivedAt: "2026-07-17T00:00:00.000Z",
        }]}
        onApplyFilter={vi.fn()}
        onLoadMore={vi.fn()}
        onReplay={vi.fn()}
      />,
    );

    expect(screen.getByRole("button", { name: "View" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Replay" })).not.toBeInTheDocument();
  });

  it("does not render reconciliation creation controls for read-only operators", () => {
    render(
      <ReconciliationMonitor
        canCreate={false}
        onApplyFilter={vi.fn()}
        onCreate={vi.fn()}
        onLoadMore={vi.fn()}
        providerAccounts={[]}
        runs={[]}
      />,
    );

    expect(screen.queryByRole("button", { name: /reconciliation run/i })).not.toBeInTheDocument();
  });
});
