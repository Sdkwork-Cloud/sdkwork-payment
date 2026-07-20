import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ReconciliationMonitor } from "../src/components/ReconciliationMonitor";
import { RefundCreateDialog, RefundMonitor } from "../src/components/RefundMonitor";
import { WebhookEventMonitor } from "../src/components/WebhookEventMonitor";
import type { PaymentIntentView, RefundView } from "../src/types/monitor-admin-types";

afterEach(() => {
  cleanup();
});

const failedRefund: RefundView = {
  amount: "88.00",
  createdAt: "2026-07-20T01:00:00.000Z",
  currencyCode: "CNY",
  id: "refund-failed",
  orderId: "order-1",
  paymentAttemptId: "attempt-1",
  paymentIntentId: "intent-1",
  providerAccountId: "account-original",
  providerCode: "stripe",
  reasonCode: "customer_request",
  refundNo: "RF-FAILED-1",
  requestedBy: "operator-1",
  requestedByType: "operator",
  status: "failed",
  updatedAt: "2026-07-20T01:01:00.000Z",
};

const succeededIntent: PaymentIntentView = {
  amount: "88.00",
  createdAt: "2026-07-20T00:00:00.000Z",
  currencyCode: "CNY",
  id: "intent-1",
  orderId: "order-1",
  ownerUserId: "owner-1",
  paymentIntentNo: "PI-EXACT-1",
  paymentMethod: "stripe_card",
  providerCode: "stripe",
  status: "succeeded",
  updatedAt: "2026-07-20T00:01:00.000Z",
};

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

  it("hides refund creation and retry controls from read-only operators", () => {
    render(
      <RefundMonitor
        canCreate={false}
        canRetry={false}
        onApplyFilter={vi.fn()}
        onLoadMore={vi.fn()}
        onRetry={vi.fn()}
        onStartCreate={vi.fn()}
        refunds={[failedRefund]}
      />,
    );

    expect(screen.queryByRole("button", { name: "New refund" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /retry refund/i })).not.toBeInTheDocument();
  });

  it("offers retry only for failed refunds and requires the exact refund number", async () => {
    const onRetry = vi.fn().mockResolvedValue(undefined);
    render(
      <RefundMonitor
        canCreate
        canRetry
        onApplyFilter={vi.fn()}
        onLoadMore={vi.fn()}
        onRetry={onRetry}
        onStartCreate={vi.fn()}
        refunds={[
          failedRefund,
          { ...failedRefund, id: "refund-succeeded", refundNo: "RF-SUCCEEDED-1", status: "succeeded" },
        ]}
      />,
    );

    const retryActions = screen.getAllByRole("button", { name: /retry refund:/i });
    expect(retryActions).toHaveLength(1);
    fireEvent.click(retryActions[0]!);

    const retryButton = screen.getByRole("button", { name: "Retry refund" });
    expect(retryButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Confirm refund number/i), {
      target: { value: "RF-WRONG" },
    });
    expect(retryButton).toBeDisabled();
    fireEvent.change(screen.getByLabelText(/Confirm refund number/i), {
      target: { value: failedRefund.refundNo },
    });
    expect(retryButton).toBeEnabled();
    fireEvent.click(retryButton);

    await waitFor(() => expect(onRetry).toHaveBeenCalledWith(failedRefund.id, failedRefund.refundNo));
  });

  it("opens a complete refund processing detail from the table", () => {
    render(
      <RefundMonitor
        canCreate={false}
        canRetry={false}
        onApplyFilter={vi.fn()}
        onLoadMore={vi.fn()}
        onRetry={vi.fn()}
        onStartCreate={vi.fn()}
        refunds={[failedRefund]}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: `View: ${failedRefund.refundNo}` }));

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(screen.getByText("Original attempt")).toBeInTheDocument();
    expect(screen.getByText(failedRefund.paymentAttemptId)).toBeInTheDocument();
    expect(screen.getAllByText(failedRefund.providerAccountId!)).toHaveLength(2);
    expect(screen.getAllByText(`operator: ${failedRefund.requestedBy}`)).toHaveLength(2);
  });

  it("applies and clears refund search filters without local pagination", async () => {
    const onApplyFilter = vi.fn().mockResolvedValue(undefined);
    render(
      <RefundMonitor
        canCreate={false}
        canRetry={false}
        onApplyFilter={onApplyFilter}
        onLoadMore={vi.fn()}
        onRetry={vi.fn()}
        onStartCreate={vi.fn()}
        refunds={[]}
      />,
    );

    fireEvent.change(screen.getByLabelText("Search"), { target: { value: "RF-2026" } });
    fireEvent.click(screen.getByRole("button", { name: "Apply filters" }));
    await waitFor(() => expect(onApplyFilter).toHaveBeenLastCalledWith({ q: "RF-2026" }));

    fireEvent.click(screen.getByRole("button", { name: "Clear filters" }));
    await waitFor(() => expect(onApplyFilter).toHaveBeenLastCalledWith({}));
  });

  it("requires the exact payment number before submitting a refund", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(
      <RefundCreateDialog
        initialIntent={succeededIntent}
        intents={[succeededIntent]}
        onOpenChange={vi.fn()}
        onSubmit={onSubmit}
        open
      />,
    );

    const submit = await screen.findByRole("button", { name: "Submit refund" });
    const confirmation = screen.getByLabelText(/Confirm payment number/i);
    expect(submit).toBeDisabled();
    fireEvent.change(confirmation, { target: { value: "PI-WRONG" } });
    expect(submit).toBeDisabled();
    fireEvent.change(confirmation, { target: { value: succeededIntent.paymentIntentNo } });
    expect(submit).toBeEnabled();
    fireEvent.click(submit);

    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({
      confirmPaymentIntentNo: succeededIntent.paymentIntentNo,
      paymentIntentId: succeededIntent.id,
      reasonCode: "customer_request",
    }));
  });

  it("rejects invalid or excessive partial refund amounts before submission", async () => {
    const onSubmit = vi.fn().mockResolvedValue(undefined);
    render(
      <RefundCreateDialog
        initialIntent={succeededIntent}
        intents={[succeededIntent]}
        onOpenChange={vi.fn()}
        onSubmit={onSubmit}
        open
      />,
    );

    const amount = screen.getByLabelText("Refund amount");
    const submit = await screen.findByRole("button", { name: "Submit refund" });
    fireEvent.change(screen.getByLabelText(/Confirm payment number/i), {
      target: { value: succeededIntent.paymentIntentNo },
    });

    fireEvent.change(amount, { target: { value: "0" } });
    expect(screen.getByText(/greater than zero/i)).toBeInTheDocument();
    expect(submit).toBeDisabled();

    fireEvent.change(amount, { target: { value: "100.00" } });
    expect(screen.getByText(/cannot exceed the original payment amount/i)).toBeInTheDocument();
    expect(submit).toBeDisabled();

    fireEvent.change(amount, { target: { value: "8.80" } });
    expect(submit).toBeEnabled();
    fireEvent.click(submit);
    await waitFor(() => expect(onSubmit).toHaveBeenCalledWith({
      amount: "8.80",
      confirmPaymentIntentNo: succeededIntent.paymentIntentNo,
      paymentIntentId: succeededIntent.id,
      reasonCode: "customer_request",
    }));
  });
});
