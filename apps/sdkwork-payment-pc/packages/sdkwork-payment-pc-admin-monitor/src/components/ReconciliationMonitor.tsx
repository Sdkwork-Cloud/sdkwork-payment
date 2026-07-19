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
import { usePaymentRecordsMessages } from "../i18n";

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

const FILTER_STATUS_VALUES: readonly ReconciliationRunStatus[] = [
  "pending",
  "queued",
  "running",
  "succeeded",
  "failed",
  "canceled",
];

const RECONCILIATION_TYPE_VALUES: readonly ReconciliationType[] = [
  "daily",
  "weekly",
  "monthly",
  "manual",
  "settlement",
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

export function ReconciliationMonitor(props: ReconciliationMonitorProps) {
  const messages = usePaymentRecordsMessages();
  const operations = messages.operations;
  const statusOptions = [
    { label: operations.filters.allStatuses, value: "" },
    ...FILTER_STATUS_VALUES.map((value) => ({
      label: operations.reconciliation.status[value],
      value,
    })),
  ];
  const providerOptions = ADMIN_PROVIDER_FILTER_OPTIONS.map((option) => ({
    ...option,
    label: option.value ? option.label : operations.filters.allProviders,
  }));
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
      setError(err instanceof Error ? err.message : operations.validation.applyFilterFailed);
    });
  }

  async function handleCreate(draft: CreateReconciliationRunDraft) {
    try {
      await props.onCreate(draft);
      setDialogOpen(false);
    } catch (err) {
      setError(err instanceof Error ? err.message : operations.validation.createReconciliationFailed);
    }
  }

  return (
    <div className="space-y-4" data-slot="payment-reconciliation-monitor">
      {props.canCreate ? (
        <div className="flex justify-end">
          <Button type="button" size="sm" onClick={() => setDialogOpen(true)} disabled={props.busy} title={operations.availability.createReconciliation}>
            {operations.actions.newReconciliation}
          </Button>
        </div>
      ) : null}

      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label={operations.filters.status} htmlFor="recon-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="recon-filter-status">
              <SelectValue placeholder={operations.filters.allStatuses} />
            </SelectTrigger>
            <SelectContent>
              {statusOptions.map((option) => (
                <SelectItem key={option.value || "all"} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label={operations.filters.provider} htmlFor="recon-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="recon-filter-provider">
              <SelectValue placeholder={operations.filters.allProviders} />
            </SelectTrigger>
            <SelectContent>
              {providerOptions.map((option) => (
                <SelectItem key={option.value || "all"} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label={operations.reconciliation.providerAccount} htmlFor="recon-filter-account">
          <Input
            id="recon-filter-account"
            value={providerAccountId}
            onChange={(event) => setProviderAccountId(event.target.value)}
            placeholder={operations.reconciliation.providerAccountPlaceholder}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label={operations.filters.search} htmlFor="recon-filter-q">
          <Input
            id="recon-filter-q"
            value={q}
            onChange={(event) => setQ(event.target.value)}
            placeholder={operations.filters.searchPlaceholder}
          />
        </AdminFieldLabel>
        <div className="col-span-full flex justify-end">
          <Button type="submit" size="sm" disabled={props.busy} title={operations.availability.applyFilter}>
            {operations.actions.applyFilter}
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
          {operations.reconciliation.empty}
          {props.canCreate ? (
            <div className="mt-3">
              <Button type="button" variant="primary" size="sm" onClick={() => setDialogOpen(true)} disabled={props.busy} title={operations.availability.createReconciliation}>
                {operations.actions.createReconciliation}
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
                <Badge variant="secondary">{operations.reconciliation.type[run.reconciliationType]}</Badge>
                <Badge variant={STATUS_VARIANT[run.status]}>{operations.reconciliation.status[run.status]}</Badge>
              </div>
              <dl className="grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-4">
                <div>
                  <dt className="inline">{operations.reconciliation.fields.account}:</dt>{" "}
                  <dd className="inline font-mono">{run.providerAccountId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.period}:</dt>{" "}
                  <dd className="inline">
                    {formatAdminTimestamp(run.periodStart)} → {formatAdminTimestamp(run.periodEnd)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.matched}:</dt> <dd className="inline">{run.matchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.mismatched}:</dt>{" "}
                  <dd className="inline">{run.mismatchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.unmatched}:</dt> <dd className="inline">{run.unmatchedCount}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.difference}:</dt>{" "}
                  <dd className="inline">
                    {formatAdminAmount(run.totalDifferenceAmount, run.currencyCode)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">{operations.reconciliation.fields.created}:</dt> <dd className="inline">{formatAdminRelativeTime(run.createdAt)}</dd>
                </div>
              </dl>
            </li>
          ))}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        label={operations.actions.loadMore}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
        summary={props.pageInfo?.totalItems
          ? messages.table.paginationSummary(props.runs.length, props.pageInfo.totalItems)
          : undefined}
      />

      <Dialog
        open={props.canCreate && dialogOpen}
        onOpenChange={(open) => {
          if (!open) setDialogOpen(false);
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{operations.actions.newReconciliation}</DialogTitle>
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
  const operations = usePaymentRecordsMessages().operations;
  const reconciliationTypeOptions = RECONCILIATION_TYPE_VALUES.map((value) => ({
    label: operations.reconciliation.type[value],
    value,
  }));
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
      setError(operations.validation.providerAccountRequired);
      return;
    }
    if (!periodStart || !periodEnd) {
      setError(operations.validation.periodRequired);
      return;
    }
    const start = new Date(periodStart);
    const end = new Date(periodEnd);
    if (Number.isNaN(start.getTime()) || Number.isNaN(end.getTime())) {
      setError(operations.validation.periodInvalid);
      return;
    }
    if (end <= start) {
      setError(operations.validation.periodOrder);
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
      setError(err instanceof Error ? err.message : operations.validation.createReconciliationFailed);
    }
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label={operations.reconciliation.form.provider} htmlFor="recon-form-provider" required>
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
        <AdminFieldLabel label={operations.reconciliation.form.reconciliationType} htmlFor="recon-form-type" required>
          <Select
            value={reconciliationType}
            onValueChange={(value) => setReconciliationType(value as ReconciliationType)}
          >
            <SelectTrigger id="recon-form-type">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {reconciliationTypeOptions.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
      </div>
      <AdminFieldLabel label={operations.reconciliation.form.providerAccount} htmlFor="recon-form-account" required>
        <Select value={providerAccountId} onValueChange={setProviderAccountId}>
          <SelectTrigger id="recon-form-account">
            <SelectValue placeholder={operations.reconciliation.form.selectAccount} />
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
        <span className="text-xs text-[var(--sdk-color-text-secondary)]">{operations.reconciliation.form.quickPresets}:</span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("yesterday")}
          disabled={props.busy}
          title={operations.reconciliation.form.yesterdayDescription}
        >
          {operations.reconciliation.form.yesterday}
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("last7days")}
          disabled={props.busy}
          title={operations.reconciliation.form.last7DaysDescription}
        >
          {operations.reconciliation.form.last7Days}
        </Button>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={() => applyPreset("lastMonth")}
          disabled={props.busy}
          title={operations.reconciliation.form.lastMonthDescription}
        >
          {operations.reconciliation.form.lastMonth}
        </Button>
      </div>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label={operations.reconciliation.form.periodStart} htmlFor="recon-form-start" required>
          <Input
            id="recon-form-start"
            type="datetime-local"
            value={periodStart}
            onChange={(event) => setPeriodStart(event.target.value)}
            required
          />
        </AdminFieldLabel>
        <AdminFieldLabel label={operations.reconciliation.form.periodEnd} htmlFor="recon-form-end" required>
          <Input
            id="recon-form-end"
            type="datetime-local"
            value={periodEnd}
            onChange={(event) => setPeriodEnd(event.target.value)}
            required
          />
        </AdminFieldLabel>
      </div>
      <AdminFieldLabel label={operations.reconciliation.form.currency} htmlFor="recon-form-currency">
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
        <Button type="button" variant="ghost" onClick={props.onCancel} disabled={props.busy} title={operations.availability.cancelCreate}>
          {operations.actions.cancel}
        </Button>
        <Button type="submit" disabled={props.busy} title={props.busy ? operations.availability.creating : operations.availability.createReconciliation}>
          {operations.actions.createRun}
        </Button>
      </div>
    </form>
  );
}
