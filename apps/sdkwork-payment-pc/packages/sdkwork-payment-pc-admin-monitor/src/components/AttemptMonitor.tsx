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
import { usePaymentRecordsMessages } from "../i18n";

export interface AttemptMonitorProps {
  attempts: readonly PaymentAttemptView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  onApplyFilter(filter: PaymentAttemptListFilter): Promise<void> | void;
  onLoadMore(): void;
}

const FILTER_STATUS_VALUES: readonly PaymentStatus[] = [
  "created",
  "pending",
  "processing",
  "succeeded",
  "failed",
  "canceled",
  "closed",
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
  const messages = usePaymentRecordsMessages();
  const operations = messages.operations;
  const statusOptions = [
    { label: operations.filters.allStatuses, value: "" },
    ...FILTER_STATUS_VALUES.map((value) => ({
      label: operations.attempts.status[value],
      value,
    })),
  ];
  const providerOptions = ADMIN_PROVIDER_FILTER_OPTIONS.map((option) => ({
    ...option,
    label: option.value ? option.label : operations.filters.allProviders,
  }));
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
      setError(err instanceof Error ? err.message : operations.validation.applyFilterFailed);
    });
  }

  function handleResetFilter() {
    setStatus("");
    setProviderCode("");
    setPaymentIntentId("");
    setQ("");
    setError(undefined);
    Promise.resolve(props.onApplyFilter({})).catch((err) => {
      setError(err instanceof Error ? err.message : operations.validation.clearFiltersFailed);
    });
  }

  return (
    <div className="space-y-4" data-slot="payment-attempt-monitor">
      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label={operations.filters.status} htmlFor="attempt-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="attempt-filter-status">
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
        <AdminFieldLabel label={operations.filters.provider} htmlFor="attempt-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="attempt-filter-provider">
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
        <AdminFieldLabel label={operations.attempts.intentIdentifier} htmlFor="attempt-filter-intent">
          <Input
            id="attempt-filter-intent"
            value={paymentIntentId}
            onChange={(event) => setPaymentIntentId(event.target.value)}
            placeholder={operations.attempts.intentPlaceholder}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label={operations.filters.search} htmlFor="attempt-filter-q">
          <Input
            id="attempt-filter-q"
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

      {props.attempts.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          {operations.attempts.empty}
          <div className="mt-3">
            <Button type="button" variant="ghost" size="sm" onClick={handleResetFilter} disabled={props.busy}>
              {operations.actions.clearFilters}
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
                <Badge variant={STATUS_VARIANT[attempt.status]}>{operations.attempts.status[attempt.status]}</Badge>
              </div>
              <dl className="grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-4">
                <div>
                  <dt className="inline">{operations.attempts.fields.intent}:</dt>{" "}
                  <dd className="inline font-mono">{attempt.paymentIntentId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.channel}:</dt>{" "}
                  <dd className="inline font-mono">{attempt.channelId || "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.amount}:</dt>{" "}
                  <dd className="inline">
                    {formatAdminAmount(attempt.amount, attempt.currencyCode)}
                  </dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.providerTransaction}:</dt>{" "}
                  <dd className="inline font-mono">{attempt.providerTransactionId ?? "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.outTradeNumber}:</dt>{" "}
                  <dd className="inline font-mono">{attempt.outTradeNo ?? "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.paidAt}:</dt> <dd className="inline">{attempt.paidAt ? formatAdminTimestamp(attempt.paidAt) : "—"}</dd>
                </div>
                <div>
                  <dt className="inline">{operations.attempts.fields.created}:</dt> <dd className="inline">{formatAdminRelativeTime(attempt.createdAt)}</dd>
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
          ? messages.table.paginationSummary(props.attempts.length, props.pageInfo.totalItems)
          : undefined}
      />
    </div>
  );
}
