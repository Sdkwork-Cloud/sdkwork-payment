/**
 * Payment intent monitor.
 *
 * Lists payment intents with filter (status / providerCode / ownerUserId / orderId / currencyCode / createdAt range / q) and
 * click-to-retrieve detail dialog. Mirrors PSP "Payments" console views
 * (Stripe Dashboard → Payments, Alipay merchant platform → transaction list).
 *
 * API matrix: list + retrieve. No create/update/delete (intents are created
 * by the order capability, not admin).
 */

import * as React from "react";
import {
  Badge,
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
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
  formatAdminAmount,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentIntentDetail,
  PaymentIntentListFilter,
  PaymentIntentView,
  PaymentProviderCode,
  PaymentStatus,
} from "../types/monitor-admin-types";

export interface IntentMonitorProps {
  intents: readonly PaymentIntentView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  selectedIntent?: PaymentIntentDetail;
  onApplyFilter(filter: PaymentIntentListFilter): Promise<void> | void;
  onLoadMore(): void;
  onSelect(intent: PaymentIntentView): Promise<void> | void;
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

export function IntentMonitor(props: IntentMonitorProps) {
  const [status, setStatus] = React.useState<string>("");
  const [ownerUserId, setOwnerUserId] = React.useState("");
  const [orderId, setOrderId] = React.useState("");
  const [q, setQ] = React.useState("");
  // Additional filters: providerCode / currencyCode / created-at range
  const [providerCode, setProviderCode] = React.useState<string>("");
  const [currencyCode, setCurrencyCode] = React.useState("");
  const [createdAtFrom, setCreatedAtFrom] = React.useState("");
  const [createdAtTo, setCreatedAtTo] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();
  const [selectedId, setSelectedId] = React.useState<string | undefined>();

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    // filter includes only non-empty fields; empty strings / unselected values are not sent to the backend
    const filter: PaymentIntentListFilter = {
      ...(status ? { status: status as PaymentStatus } : {}),
      ...(ownerUserId.trim() ? { ownerUserId: ownerUserId.trim() } : {}),
      ...(orderId.trim() ? { orderId: orderId.trim() } : {}),
      ...(providerCode ? { providerCode: providerCode as PaymentProviderCode } : {}),
      ...(currencyCode.trim() ? { currencyCode: currencyCode.trim() } : {}),
      ...(createdAtFrom ? { createdAtFrom: new Date(createdAtFrom).toISOString() } : {}),
      ...(createdAtTo ? { createdAtTo: new Date(createdAtTo).toISOString() } : {}),
      ...(q.trim() ? { q: q.trim() } : {}),
    };
    Promise.resolve(props.onApplyFilter(filter)).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to apply filter.");
    });
  }

  function handleResetFilter() {
    setStatus("");
    setOwnerUserId("");
    setOrderId("");
    setQ("");
    setProviderCode("");
    setCurrencyCode("");
    setCreatedAtFrom("");
    setCreatedAtTo("");
    setError(undefined);
    Promise.resolve(props.onApplyFilter({})).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to clear filters.");
    });
  }

  async function handleSelect(intent: PaymentIntentView) {
    setSelectedId(intent.id);
    try {
      await props.onSelect(intent);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load intent detail.");
    }
  }

  return (
    <div className="space-y-4" data-slot="payment-intent-monitor">
      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label="Status" htmlFor="intent-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="intent-filter-status">
              <SelectValue placeholder="All statuses" />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((option) => (
                <SelectItem key={option.label} value={String(option.value)}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Provider" htmlFor="intent-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="intent-filter-provider">
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
        <AdminFieldLabel label="Owner user ID" htmlFor="intent-filter-owner">
          <Input
            id="intent-filter-owner"
            value={ownerUserId}
            onChange={(event) => setOwnerUserId(event.target.value)}
            placeholder="Filter by owner"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Order ID" htmlFor="intent-filter-order">
          <Input
            id="intent-filter-order"
            value={orderId}
            onChange={(event) => setOrderId(event.target.value)}
            placeholder="Filter by order"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Currency" htmlFor="intent-filter-currency">
          <Input
            id="intent-filter-currency"
            value={currencyCode}
            onChange={(event) => setCurrencyCode(event.target.value)}
            placeholder="e.g., CNY"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Created from" htmlFor="intent-filter-created-from">
          <Input
            id="intent-filter-created-from"
            type="datetime-local"
            value={createdAtFrom}
            onChange={(event) => setCreatedAtFrom(event.target.value)}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Created to" htmlFor="intent-filter-created-to">
          <Input
            id="intent-filter-created-to"
            type="datetime-local"
            value={createdAtTo}
            onChange={(event) => setCreatedAtTo(event.target.value)}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Search" htmlFor="intent-filter-q">
          <Input
            id="intent-filter-q"
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

      {props.intents.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No payment intents found. Adjust the filter or wait for new transactions.
          <div className="mt-3">
            <Button type="button" variant="ghost" size="sm" onClick={handleResetFilter} disabled={props.busy}>
              Clear filters
            </Button>
          </div>
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.intents.map((intent) => (
            <li
              key={intent.id}
              className={
                "flex flex-col gap-2 p-4 sm:flex-row sm:items-center sm:justify-between " +
                (selectedId === intent.id ? "bg-[var(--sdk-color-bg-subtle)]" : "")
              }
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-mono text-sm font-medium text-[var(--sdk-color-text)]">
                    {intent.paymentIntentNo}
                  </span>
                  <Badge variant="outline" className="font-mono">
                    {intent.providerCode}
                  </Badge>
                  <Badge variant={STATUS_VARIANT[intent.status]}>{intent.status}</Badge>
                  <Badge variant="outline">{intent.attempts?.length ?? 0} attempts</Badge>
                </div>
                <dl className="mt-2 grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-4">
                  <div>
                    <dt className="inline">Order:</dt> <dd className="inline">{intent.orderId || "—"}</dd>
                  </div>
                  <div>
                    <dt className="inline">Owner:</dt> <dd className="inline">{intent.ownerUserId || "—"}</dd>
                  </div>
                  <div>
                    <dt className="inline">Amount:</dt>{" "}
                    <dd className="inline">
                      {formatAdminAmount(intent.amount, intent.currencyCode)}
                    </dd>
                  </div>
                  <div>
                    <dt className="inline">Method:</dt>{" "}
                    <dd className="inline">{intent.paymentMethod || "—"}</dd>
                  </div>
                </dl>
                <p className="mt-1 text-xs text-[var(--sdk-color-text-muted)]">
                  Created {formatAdminRelativeTime(intent.createdAt)} · Updated {formatAdminRelativeTime(intent.updatedAt)}
                </p>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => void handleSelect(intent)}
                  disabled={props.busy}
                  title={props.busy ? "Cannot view detail while another operation is in progress" : "View payment intent detail"}
                >
                  View detail
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
      />

      <Dialog
        open={Boolean(props.selectedIntent)}
        onOpenChange={(open) => {
          if (!open) setSelectedId(undefined);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Payment intent detail</DialogTitle>
          </DialogHeader>
          {props.selectedIntent ? <IntentDetail detail={props.selectedIntent} /> : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

function IntentDetail({ detail }: { detail: PaymentIntentDetail }) {
  return (
    <div className="space-y-4">
      <dl className="grid grid-cols-1 gap-x-6 gap-y-2 text-sm sm:grid-cols-2">
        <DetailRow label="Intent No" value={detail.paymentIntentNo} mono />
        <DetailRow label="Status" value={detail.status} />
        <DetailRow label="Order ID" value={detail.orderId || "—"} mono />
        <DetailRow label="Owner user ID" value={detail.ownerUserId || "—"} mono />
        <DetailRow label="Payment method" value={detail.paymentMethod || "—"} />
        <DetailRow label="Provider" value={detail.providerCode} />
        <DetailRow label="Amount" value={formatAdminAmount(detail.amount, detail.currencyCode)} />
        <DetailRow label="Created at" value={formatAdminRelativeTime(detail.createdAt)} />
        <DetailRow label="Updated at" value={formatAdminTimestamp(detail.updatedAt)} />
      </dl>
      <div>
        <h4 className="mb-2 text-sm font-semibold text-[var(--sdk-color-text)]">Attempts</h4>
        {detail.attempts && detail.attempts.length > 0 ? (
          <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)] text-xs">
            {detail.attempts.map((attempt) => (
              <li key={attempt.id} className="grid grid-cols-2 gap-2 p-2 sm:grid-cols-4">
                <span className="font-mono">{attempt.attemptNo}</span>
                <Badge variant="outline" className="font-mono">
                  {attempt.providerCode}
                </Badge>
                <Badge variant={STATUS_VARIANT[attempt.status]}>{attempt.status}</Badge>
                <span>
                  {formatAdminAmount(attempt.amount, attempt.currencyCode)}
                </span>
                {attempt.providerTransactionId ? (
                  <span className="col-span-2 text-[var(--sdk-color-text-muted)]">
                    PSP tx: {attempt.providerTransactionId}
                  </span>
                ) : null}
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-[var(--sdk-color-text-secondary)]">No attempts recorded for this intent.</p>
        )}
      </div>
      {detail.metadata && Object.keys(detail.metadata).length > 0 ? (
        <div>
          <h4 className="mb-2 text-sm font-semibold text-[var(--sdk-color-text)]">Metadata</h4>
          <pre className="overflow-x-auto rounded-md border border-[var(--sdk-color-border-subtle)] bg-[var(--sdk-color-bg-subtle)] p-3 text-xs text-[var(--sdk-color-text-secondary)]">
            {JSON.stringify(detail.metadata, null, 2)}
          </pre>
        </div>
      ) : null}
    </div>
  );
}

function DetailRow({ label, value, mono }: { label: string; value: string; mono?: boolean }) {
  return (
    <div>
      <dt className="text-xs text-[var(--sdk-color-text-muted)]">{label}</dt>
      <dd className={mono ? "font-mono text-sm" : "text-sm"}>{value}</dd>
    </div>
  );
}
