/**
 * Reconciliation run monitor.
 *
 * Lists reconciliation runs with filter (status / providerCode /
 * providerAccountId / q) and create action. Mirrors PSP settlement/recon
 * consoles (Stripe Dashboard → Reports → Reconciliation, Alipay merchant
 * platform → reconciliation center).
 *
 * API matrix: list + create. No retrieve/update/delete — runs are immutable
 * once created (status transitions are owned by the reconciliation executor).
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
  ADMIN_PROVIDER_FORM_OPTIONS,
  ADMIN_PROVIDER_LABEL,
  formatAdminAmount,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  CreateReconciliationRunDraft,
  PaymentProviderCode,
  ReconciliationProviderAccountOption,
  ReconciliationRunListFilter,
  ReconciliationRunStatus,
  ReconciliationRunView,
  ReconciliationType,
} from "../types/monitor-admin-types";

// re-export so consumers can import from a single entry point (kept consistent with monitor-admin-types)
export type { ReconciliationProviderAccountOption };

export interface ReconciliationMonitorProps {
  runs: readonly ReconciliationRunView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  canCreate: boolean;
  // The create-run form requires selecting from configured provider accounts, so the dropdown data source is injected by the caller
  providerAccounts: readonly ReconciliationProviderAccountOption[];
  onApplyFilter(filter: ReconciliationRunListFilter): Promise<void> | void;
  onLoadMore(): void;
  onCreate(draft: CreateReconciliationRunDraft): Promise<void> | void;
}

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: ReconciliationRunStatus | "" }> = [
  { label: "All statuses", value: "" },
  { label: "Pending", value: "pending" },
  { label: "Queued", value: "queued" },
  { label: "Running", value: "running" },
  { label: "Succeeded", value: "succeeded" },
  { label: "Failed", value: "failed" },
  { label: "Canceled", value: "canceled" },
];

const RECONCILIATION_TYPE_OPTIONS: ReadonlyArray<{ label: string; value: ReconciliationType }> = [
  { label: "Daily", value: "daily" },
  { label: "Weekly", value: "weekly" },
  { label: "Monthly", value: "monthly" },
  { label: "Manual", value: "manual" },
  { label: "Settlement", value: "settlement" },
];

// Supported currency list: currencyCode is now a dropdown, limited to commonly used currencies
const CURRENCY_OPTIONS: ReadonlyArray<{ label: string; value: string }> = [
  { label: "CNY", value: "CNY" },
  { label: "USD", value: "USD" },
  { label: "EUR", value: "EUR" },
  { label: "HKD", value: "HKD" },
  { label: "JPY", value: "JPY" },
  { label: "GBP", value: "GBP" },
];

// Period quick preset: formats the datetime-local value as YYYY-MM-DDTHH:mm
function toDatetimeLocalValue(date: Date): string {
  return date.toISOString().slice(0, 16);
}

// Returns 00:00 of the local-timezone day (avoids date drift caused by UTC offset)
function startOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
}

// Returns 23:59 of the local-timezone day
function endOfLocalDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 0, 0);
}

const STATUS_VARIANT: Record<ReconciliationRunStatus, "default" | "success" | "warning" | "danger" | "secondary"> = {
  pending: "secondary",
  queued: "warning",
  running: "warning",
  succeeded: "success",
  failed: "danger",
  canceled: "secondary",
};

const RECONCILIATION_TYPE_LABEL: Record<ReconciliationType, string> = {
  daily: "Daily",
  weekly: "Weekly",
  monthly: "Monthly",
  manual: "Manual",
  settlement: "Settlement",
};

export function ReconciliationMonitor(props: ReconciliationMonitorProps) {
  const [dialogOpen, setDialogOpen] = React.useState(false);
  const [status, setStatus] = React.useState<string>("");
  const [providerCode, setProviderCode] = React.useState<string>("");
  const [providerAccountId, setProviderAccountId] = React.useState("");
  const [q, setQ] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    const filter: ReconciliationRunListFilter = {
      ...(status ? { status: status as ReconciliationRunStatus } : {}),
      ...(providerCode ? { providerCode: providerCode as PaymentProviderCode } : {}),
      ...(providerAccountId.trim() ? { providerAccountId: providerAccountId.trim() } : {}),
      ...(q.trim() ? { q: q.trim() } : {}),
    };
    Promise.resolve(props.onApplyFilter(filter)).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to apply filter.");
    });
  }

  async function handleCreate(draft: CreateReconciliationRunDraft) {
    try {
      await props.onCreate(draft);
      setDialogOpen(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create reconciliation run.");
    }
  }

  return (
    <div className="space-y-4" data-slot="payment-reconciliation-monitor">
      {props.canCreate ? (
        <div className="flex justify-end">
          <Button type="button" size="sm" onClick={() => setDialogOpen(true)} disabled={props.busy} title={props.busy ? "Cannot create a reconciliation run while another operation is in progress" : "Create a new reconciliation run"}>
            New reconciliation run
          </Button>
        </div>
      ) : null}

      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label="Status" htmlFor="recon-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="recon-filter-status">
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
        <AdminFieldLabel label="Provider" htmlFor="recon-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="recon-filter-provider">
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
        <AdminFieldLabel label="Provider account" htmlFor="recon-filter-account">
          <Input
            id="recon-filter-account"
            value={providerAccountId}
            onChange={(event) => setProviderAccountId(event.target.value)}
            placeholder="Filter by account ID"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Search" htmlFor="recon-filter-q">
          <Input
            id="recon-filter-q"
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

      {props.runs.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No reconciliation runs found. Create one to start a new reconciliation cycle.
          {props.canCreate ? (
            <div className="mt-3">
              <Button type="button" variant="primary" size="sm" onClick={() => setDialogOpen(true)} disabled={props.busy} title={props.busy ? "Cannot create a reconciliation run while another operation is in progress" : "Create a new reconciliation run"}>
                Create reconciliation run
              </Button>
            </div>
          ) : null}
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.runs.map((run) => (
            <li key={run.id} className="flex flex-col gap-2 p-4">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-mono text-sm font-medium text-[var(--sdk-color-text)]">
                  {run.runNo}
                </span>
                <Badge variant="outline" className="font-mono">
                  {ADMIN_PROVIDER_LABEL[run.providerCode]}
                </Badge>
                <Badge variant="secondary">{RECONCILIATION_TYPE_LABEL[run.reconciliationType]}</Badge>
                <Badge variant={STATUS_VARIANT[run.status]}>{run.status}</Badge>
              </div>
              <dl className="grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-4">
                <div>
                  <dt className="inline">Account:</dt>{" "}
                  <dd className="inline font-mono">{run.providerAccountId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">Period:</dt>{" "}
                  <dd className="inline">
                    {formatAdminTimestamp(run.periodStart)} → {formatAdminTimestamp(run.periodEnd)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">Matched:</dt> <dd className="inline">{run.matchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">Mismatched:</dt>{" "}
                  <dd className="inline">{run.mismatchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">Unmatched:</dt> <dd className="inline">{run.unmatchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">Difference:</dt>{" "}
                  <dd className="inline">
                    {formatAdminAmount(run.totalDifferenceAmount, run.currencyCode)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">Created:</dt> <dd className="inline">{formatAdminRelativeTime(run.createdAt)}</dd>
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

      <Dialog
        open={props.canCreate && dialogOpen}
        onOpenChange={(open) => {
          if (!open) setDialogOpen(false);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>New reconciliation run</DialogTitle>
          </DialogHeader>
          <ReconciliationRunForm
            providerAccounts={props.providerAccounts}
            onCancel={() => setDialogOpen(false)}
            onSubmit={handleCreate}
            busy={props.busy}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface ReconciliationRunFormProps {
  // providerAccount in the form is now a dropdown; the parent component must inject the account list
  providerAccounts: readonly ReconciliationProviderAccountOption[];
  onCancel(): void;
  onSubmit(draft: CreateReconciliationRunDraft): Promise<void> | void;
  busy?: boolean;
}

function ReconciliationRunForm(props: ReconciliationRunFormProps) {
  const [providerCode, setProviderCode] = React.useState<PaymentProviderCode>("alipay");
  const [providerAccountId, setProviderAccountId] = React.useState("");
  const [reconciliationType, setReconciliationType] =
    React.useState<ReconciliationType>("manual");
  const [periodStart, setPeriodStart] = React.useState("");
  const [periodEnd, setPeriodEnd] = React.useState("");
  const [currencyCode, setCurrencyCode] = React.useState("CNY");
  const [error, setError] = React.useState<string | undefined>();

  // Only show accounts under the current providerCode: prevents selecting an account that does not match the provider
  const providerAccountOptions = React.useMemo(
    () => props.providerAccounts.filter((account) => account.providerCode === providerCode),
    [props.providerAccounts, providerCode],
  );

  // Clear the selected account when switching provider: keeps providerAccountId and providerCode always in sync
  function handleProviderCodeChange(value: string) {
    setProviderCode(value as PaymentProviderCode);
    setProviderAccountId("");
  }

  // Period quick preset: sets both periodStart and periodEnd on click
  function applyPreset(preset: "yesterday" | "last7days" | "lastMonth") {
    const now = new Date();
    let start: Date;
    let end: Date;
    if (preset === "yesterday") {
      const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
      start = startOfLocalDay(yesterday);
      end = endOfLocalDay(yesterday);
    } else if (preset === "last7days") {
      const sevenDaysAgo = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 7);
      start = startOfLocalDay(sevenDaysAgo);
      end = startOfLocalDay(now);
    } else {
      // lastMonth: 00:00 on the 1st of last month → 00:00 on the 1st of this month
      start = new Date(now.getFullYear(), now.getMonth() - 1, 1, 0, 0, 0, 0);
      end = new Date(now.getFullYear(), now.getMonth(), 1, 0, 0, 0, 0);
    }
    setPeriodStart(toDatetimeLocalValue(start));
    setPeriodEnd(toDatetimeLocalValue(end));
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!providerAccountId) {
      setError("Provider account is required.");
      return;
    }
    if (!periodStart || !periodEnd) {
      setError("Period start and end are required.");
      return;
    }
    const start = new Date(periodStart);
    const end = new Date(periodEnd);
    if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
      setError("Period start and end must be valid dates.");
      return;
    }
    if (end <= start) {
      setError("Period end must be after period start.");
      return;
    }
    try {
      await props.onSubmit({
        providerCode,
        providerAccountId,
        reconciliationType,
        periodStart: start.toISOString(),
        periodEnd: end.toISOString(),
        currencyCode,
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create reconciliation run.");
    }
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Provider" htmlFor="recon-form-provider" required>
          <Select
            value={providerCode}
            onValueChange={handleProviderCodeChange}
          >
            <SelectTrigger id="recon-form-provider">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ADMIN_PROVIDER_FORM_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Reconciliation type" htmlFor="recon-form-type" required>
          <Select
            value={reconciliationType}
            onValueChange={(value) => setReconciliationType(value as ReconciliationType)}
          >
            <SelectTrigger id="recon-form-type">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {RECONCILIATION_TYPE_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
      </div>
      <AdminFieldLabel label="Provider account" htmlFor="recon-form-account" required>
        <Select value={providerAccountId} onValueChange={setProviderAccountId}>
          <SelectTrigger id="recon-form-account">
            <SelectValue placeholder="Select an account" />
          </SelectTrigger>
          <SelectContent>
            {providerAccountOptions.map((account) => (
              <SelectItem key={account.id} value={account.id}>
                {account.accountNo} ({account.providerCode})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminFieldLabel>
      {/* Period quick preset: auto-fills periodStart/periodEnd on click */}
      <div className="flex flex-wrap items-center gap-2">
        <span className="text-xs text-[var(--sdk-color-text-secondary)]">Quick presets:</span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("yesterday")}
          disabled={props.busy}
          title="Set period to yesterday 00:00 – 23:59"
        >
          Yesterday
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("last7days")}
          disabled={props.busy}
          title="Set period to 7 days ago 00:00 – today 00:00"
        >
          Last 7 days
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("lastMonth")}
          disabled={props.busy}
          title="Set period to last month 1st 00:00 – this month 1st 00:00"
        >
          Last month
        </Button>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Period start" htmlFor="recon-form-start" required>
          <Input
            id="recon-form-start"
            type="datetime-local"
            value={periodStart}
            onChange={(event) => setPeriodStart(event.target.value)}
            required
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Period end" htmlFor="recon-form-end" required>
          <Input
            id="recon-form-end"
            type="datetime-local"
            value={periodEnd}
            onChange={(event) => setPeriodEnd(event.target.value)}
            required
          />
        </AdminFieldLabel>
      </div>
      <AdminFieldLabel label="Currency" htmlFor="recon-form-currency">
        <Select value={currencyCode} onValueChange={setCurrencyCode}>
          <SelectTrigger id="recon-form-currency">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {CURRENCY_OPTIONS.map((option) => (
              <SelectItem key={option.value} value={option.value}>
                {option.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminFieldLabel>
      {error ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {error}
        </div>
      ) : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={props.onCancel} disabled={props.busy} title={props.busy ? "Cannot cancel while a run is being created" : "Cancel reconciliation run creation"}>
          Cancel
        </Button>
        <Button type="submit" disabled={props.busy} title={props.busy ? "A reconciliation run is being created..." : "Create this reconciliation run"}>
          Create run
        </Button>
      </div>
    </form>
  );
}
