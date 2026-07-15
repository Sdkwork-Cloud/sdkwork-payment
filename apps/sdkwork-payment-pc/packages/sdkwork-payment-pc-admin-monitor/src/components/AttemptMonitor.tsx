/**
 * Payment attempt monitor.
 *
 * Lists payment attempts with filter (status / providerCode /
 * paymentIntentId / q). Attempts are the PSP-facing side of a payment —
 * each intent may produce multiple attempts (retries, provider routing).
 *
 * API matrix: list only (no retrieve/create/update/delete — attempts are
 * created by the payment executor, not admin).
 */

import * as React from "react";
import {
  Badge,
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  ADMIN_PROVIDER_FILTER_OPTIONS,
  ADMIN_PROVIDER_LABEL,
  formatAdminAmount,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentAttemptListFilter,
  PaymentAttemptView,
  PaymentProviderCode,
  PaymentStatus,
} from "../types/monitor-admin-types";

export interface AttemptMonitorProps {
  attempts: readonly PaymentAttemptView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  onApplyFilter(filter: PaymentAttemptListFilter): Promise<void> | void;
  onLoadMore(): void;
}

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: PaymentStatus | "" }> = [
  { label: "All statuses", value: "" },
  { label: "Created", value: "created" },
  { label: "Pending", value: "pending" },
  { label: "Processing", value: "processing" },
  { label: "Succeeded", value: "succeeded" },
  { label: "Failed", value: "failed" },
  { label: "Canceled", value: "canceled" },
  { label: "Closed", value: "closed" },
];

const STATUS_VARIANT: Record<PaymentStatus, "default" | "success" | "warning" | "danger" | "secondary"> = {
  created: "default",
  pending: "warning",
  processing: "warning",
  succeeded: "success",
  failed: "danger",
  canceled: "secondary",
  closed: "secondary",
  refunding: "warning",
  refunded: "success",
};

export function AttemptMonitor(props: AttemptMonitorProps) {
  const [status, setStatus] = React.useState<string>("");
  const [providerCode, setProviderCode] = React.useState<string>("");
  const [paymentIntentId, setPaymentIntentId] = React.useState("");
  const [q, setQ] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    const filter: PaymentAttemptListFilter = {
      ...(status ? { status: status as PaymentStatus } : {}),
      ...(providerCode ? { providerCode: providerCode as PaymentProviderCode } : {}),
      ...(paymentIntentId.trim() ? { paymentIntentId: paymentIntentId.trim() } : {}),
      ...(q.trim() ? { q: q.trim() } : {}),
    };
    Promise.resolve(props.onApplyFilter(filter)).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to apply filter.");
    });
  }

  function handleResetFilter() {
    setStatus("");
    setProviderCode("");
    setPaymentIntentId("");
    setQ("");
    setError(undefined);
    Promise.resolve(props.onApplyFilter({})).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to clear filters.");
    });
  }

  return (
    <div className="space-y-4" data-slot="payment-attempt-monitor">
      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label="Status" htmlFor="attempt-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="attempt-filter-status">
              <SelectValue placeholder="All statuses" />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((option) => (
                <SelectItem key={option.label} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Provider" htmlFor="attempt-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="attempt-filter-provider">
              <SelectValue placeholder="All providers" />
            </SelectTrigger>
            <SelectContent>
              {ADMIN_PROVIDER_FILTER_OPTIONS.map((option) => (
                <SelectItem key={option.label} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Intent ID" htmlFor="attempt-filter-intent">
          <Input
            id="attempt-filter-intent"
            value={paymentIntentId}
            onChange={(event) => setPaymentIntentId(event.target.value)}
            placeholder="Filter by intent"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Search" htmlFor="attempt-filter-q">
          <Input
            id="attempt-filter-q"
            value={q}
            onChange={(event) => setQ(event.target.value)}
            placeholder="Free-text search"
          />
        </AdminFieldLabel>
        <div className="col-span-full flex justify-end">
          <Button type="submit" size="sm" disabled={props.busy} title={props.busy ? "Cannot apply filter while another operation is in progress" : "Apply the current filter"}>
            Apply filter
          </Button>
        </div>
      </form>

      {error ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {error}
        </div>
      ) : null}

      {props.attempts.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No payment attempts found. Adjust the filter or wait for new transactions.
          <div className="mt-3">
            <Button type="button" variant="ghost" size="sm" onClick={handleResetFilter} disabled={props.busy}>
              Clear filters
            </Button>
          </div>
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.attempts.map((attempt) => (
            <li key={attempt.id} className="flex flex-col gap-2 p-4">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-sm font-medium text-[var(--sdk-color-text)]">
                  {attempt.attemptNo}
                </span>
                <Badge variant="outline" className="font-mono">
                  {ADMIN_PROVIDER_LABEL[attempt.providerCode]}
                </Badge>
                <Badge variant={STATUS_VARIANT[attempt.status]}>{attempt.status}</Badge>
              </div>
              <dl className="grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-4">
                <div>
                  <dt className="inline">Intent:</dt>{" "}
                  <dd className="inline font-mono">{attempt.paymentIntentId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Channel:</dt>{" "}
                  <dd className="inline font-mono">{attempt.channelId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Amount:</dt>{" "}
                  <dd className="inline">
                    {formatAdminAmount(attempt.amount, attempt.currencyCode)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">PSP tx:</dt>{" "}
                  <dd className="inline font-mono">{attempt.providerTransactionId ?? "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Out trade no:</dt>{" "}
                  <dd className="inline font-mono">{attempt.outTradeNo ?? "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Paid at:</dt> <dd className="inline">{attempt.paidAt ? formatAdminTimestamp(attempt.paidAt) : "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Created:</dt> <dd className="inline">{formatAdminRelativeTime(attempt.createdAt)}</dd>
                </div>
              </dl>
            </li>
          ))}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
      />
    </div>
  );
}
