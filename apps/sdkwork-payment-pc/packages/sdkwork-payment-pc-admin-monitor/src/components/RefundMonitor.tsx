import * as React from "react";
import { ChevronRight, Eye, Plus, RotateCcw, Search, ShieldCheck, X } from "lucide-react";
import {
  Badge,
  Button,
  DataTable,
  DescriptionDetails,
  DescriptionItem,
  DescriptionList,
  DescriptionTerm,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Drawer,
  DrawerBody,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  type DataTableColumn,
} from "@sdkwork/ui-pc-react";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import {
  AdminFieldLabel,
  formatAdminAmount,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  PaymentProviderIcon,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import { usePaymentRecordsMessages } from "../i18n";
import type {
  CreateRefundDraft,
  PaymentIntentView,
  RefundListFilter,
  RefundReasonCode,
  RefundStatus,
  RefundView,
} from "../types/monitor-admin-types";
import {
  REFUND_REASON_VALUES,
  REFUND_STATUS_VALUES,
} from "../types/monitor-admin-types";

export interface RefundMonitorProps {
  refunds: readonly RefundView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  canCreate: boolean;
  canRetry: boolean;
  onApplyFilter(filter: RefundListFilter): Promise<void> | void;
  onLoadMore(): void;
  onStartCreate(): void;
  onRetry(refundId: string, confirmRefundNo: string): Promise<void> | void;
}

const REFUND_BADGE_VARIANT: Record<
  RefundStatus,
  "default" | "success" | "warning" | "danger" | "secondary"
> = {
  closed: "secondary",
  failed: "danger",
  processing: "warning",
  submitted: "secondary",
  succeeded: "success",
};

export function RefundMonitor(props: RefundMonitorProps) {
  const messages = usePaymentRecordsMessages();
  const refunds = messages.operations.refunds;
  const [status, setStatus] = React.useState<RefundStatus | "all">("all");
  const [q, setQ] = React.useState("");
  const [retrying, setRetrying] = React.useState<RefundView>();
  const [viewing, setViewing] = React.useState<RefundView>();
  const [retryConfirmation, setRetryConfirmation] = React.useState("");
  const [error, setError] = React.useState<string>();
  const hasActiveFilters = status !== "all" || q.trim().length > 0;
  const statusSummary = React.useMemo(() => {
    const failed = props.refunds.filter((refund) => refund.status === "failed").length;
    const inFlight = props.refunds.filter((refund) =>
      refund.status === "submitted" || refund.status === "processing"
    ).length;
    const completed = props.refunds.filter((refund) => refund.status === "succeeded").length;
    return { completed, failed, inFlight, loaded: props.refunds.length };
  }, [props.refunds]);

  async function applyFilter(nextStatus: RefundStatus | "all", nextQuery: string) {
    await props.onApplyFilter({
      ...(nextStatus !== "all" ? { status: nextStatus } : {}),
      ...(nextQuery.trim() ? { q: nextQuery.trim() } : {}),
    });
  }

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    Promise.resolve(applyFilter(status, q)).catch((caught) => {
      setError(caught instanceof Error ? caught.message : messages.operations.validation.applyFilterFailed);
    });
  }

  function handleClearFilters() {
    setStatus("all");
    setQ("");
    setError(undefined);
    Promise.resolve(applyFilter("all", "")).catch((caught) => {
      setError(caught instanceof Error ? caught.message : messages.operations.validation.clearFiltersFailed);
    });
  }

  async function handleRetry() {
    if (!retrying || retryConfirmation.trim() !== retrying.refundNo) {
      setError(messages.operations.validation.refundConfirmationRequired);
      return;
    }
    setError(undefined);
    try {
      await props.onRetry(retrying.id, retryConfirmation.trim());
      setRetrying(undefined);
      setRetryConfirmation("");
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : messages.operations.validation.retryRefundFailed);
    }
  }

  const columns: DataTableColumn<RefundView>[] = [
    {
      id: "refund",
      header: messages.workspace.tabs.refunds,
      width: "20%",
      cell: (refund) => (
        <div className="relative min-w-[7.5rem] space-y-1 pr-4 sm:min-w-[10rem] sm:pr-0">
          <div className="break-words font-mono text-xs font-semibold text-[var(--sdk-color-text-primary)] sm:text-sm">
            {refund.refundNo.replace(/-([^-]+)$/u, "-\u200B$1")}
          </div>
          <div className="text-xs text-[var(--sdk-color-text-muted)]">
            {refunds.reason[refund.reasonCode ?? "other"]}
          </div>
          <ChevronRight aria-hidden="true" className="absolute right-0 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--sdk-color-text-muted)] sm:hidden" />
        </div>
      ),
    },
    {
      id: "references",
      header: `${refunds.fields.payment} / ${refunds.fields.order}`,
      width: "24%",
      headerProps: { className: "hidden md:!table-cell" },
      cellProps: { className: "hidden md:!table-cell" },
      cell: (refund) => (
        <div className="min-w-[11rem] space-y-1 font-mono text-xs">
          <div className="truncate text-[var(--sdk-color-text-primary)]" title={refund.paymentIntentId}>
            {refund.paymentIntentId}
          </div>
          <div className="truncate text-[var(--sdk-color-text-muted)]" title={refund.orderId}>
            {refund.orderId}
          </div>
        </div>
      ),
    },
    {
      id: "account",
      header: refunds.fields.providerAccount,
      width: "18%",
      headerProps: { className: "hidden lg:!table-cell" },
      cellProps: { className: "hidden lg:!table-cell" },
      cell: (refund) => (
        <div className="flex min-w-[9rem] items-center gap-2">
          <PaymentProviderIcon providerCode={refund.providerCode} size="sm" />
          <div className="min-w-0">
            <div className="text-sm text-[var(--sdk-color-text-primary)]">{refund.providerCode}</div>
            <div className="truncate font-mono text-xs text-[var(--sdk-color-text-muted)]">
              {refund.providerAccountId ?? "--"}
            </div>
          </div>
        </div>
      ),
    },
    {
      align: "right",
      id: "amount",
      header: refunds.fields.amount,
      width: "14%",
      headerProps: { className: "whitespace-nowrap" },
      cellProps: { className: "whitespace-nowrap" },
      cell: (refund) => (
        <span className="font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {formatAdminAmount(refund.amount, refund.currencyCode)}
        </span>
      ),
    },
    {
      id: "status",
      header: refunds.fields.status,
      width: "12%",
      headerProps: { className: "whitespace-nowrap" },
      cellProps: { className: "whitespace-nowrap" },
      cell: (refund) => (
        <Badge className="whitespace-nowrap" variant={REFUND_BADGE_VARIANT[refund.status]}>
          {refunds.status[refund.status]}
        </Badge>
      ),
    },
    {
      id: "created",
      header: refunds.fields.created,
      width: "12%",
      headerProps: { className: "hidden xl:!table-cell" },
      cellProps: { className: "hidden xl:!table-cell" },
      cell: (refund) => (
        <div className="text-xs">
          <div>{formatAdminRelativeTime(refund.createdAt)}</div>
          <div className="mt-1 text-[var(--sdk-color-text-muted)]">
            {refund.requestedByType}: {refund.requestedBy ?? "--"}
          </div>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4" data-slot="payment-refund-monitor">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--sdk-color-border-subtle)] pb-3">
        <p className="max-w-3xl text-sm text-[var(--sdk-color-text-secondary)]">
          {refunds.createDescription}
        </p>
        {props.canCreate ? (
          <Button type="button" size="sm" onClick={props.onStartCreate} disabled={props.busy}>
            <Plus aria-hidden="true" className="mr-2 h-4 w-4" />
            {messages.operations.actions.newRefund}
          </Button>
        ) : null}
      </div>

      <dl className="grid grid-cols-2 border-y border-[var(--sdk-color-border-subtle)] md:grid-cols-4" aria-label={refunds.summary.label}>
        {([
          [refunds.summary.loaded, statusSummary.loaded, ""],
          [refunds.summary.needsAttention, statusSummary.failed, "text-[var(--sdk-color-text-error)]"],
          [refunds.summary.inFlight, statusSummary.inFlight, "text-[var(--sdk-color-text-warning)]"],
          [refunds.summary.completed, statusSummary.completed, "text-[var(--sdk-color-text-success)]"],
        ] as const).map(([label, value, tone], index) => (
          <div className={`px-3 py-2.5 ${index % 2 ? "border-l" : ""} ${index > 1 ? "border-t md:border-t-0" : ""} md:border-l md:first:border-l-0 border-[var(--sdk-color-border-subtle)]`} key={label}>
            <dt className="text-xs text-[var(--sdk-color-text-muted)]">{label}</dt>
            <dd className={`mt-0.5 text-lg font-semibold tabular-nums ${tone || "text-[var(--sdk-color-text-primary)]"}`}>{value}</dd>
          </div>
        ))}
      </dl>

      <form className="flex flex-wrap items-end gap-3" onSubmit={handleApply}>
        <AdminFieldLabel className="min-w-[12rem]" label={messages.operations.filters.status} htmlFor="refund-filter-status">
          <Select disabled={props.busy} value={status} onValueChange={(value) => setStatus(value as RefundStatus | "all")}>
            <SelectTrigger id="refund-filter-status"><SelectValue /></SelectTrigger>
            <SelectContent>
              <SelectItem value="all">{messages.operations.filters.allStatuses}</SelectItem>
              {REFUND_STATUS_VALUES.map((value) => (
                <SelectItem key={value} value={value}>{refunds.status[value]}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel className="min-w-[16rem] flex-1" label={messages.operations.filters.search} htmlFor="refund-filter-search">
          <div className="relative">
            <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--sdk-color-text-muted)]" />
            <Input id="refund-filter-search" className="pl-9" disabled={props.busy} value={q} onChange={(event) => setQ(event.target.value)} placeholder={messages.operations.filters.searchPlaceholder} />
          </div>
        </AdminFieldLabel>
        <Button type="submit" size="sm" disabled={props.busy}>{messages.operations.actions.applyFilter}</Button>
        {hasActiveFilters ? (
          <Button type="button" size="sm" variant="ghost" disabled={props.busy} onClick={handleClearFilters}>
            <X aria-hidden="true" className="mr-2 h-4 w-4" />
            {messages.operations.actions.clearFilters}
          </Button>
        ) : null}
      </form>

      {error ? <div role="alert" className="border-l-2 border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-error)]">{error}</div> : null}

      <DataTable
        columns={columns}
        density="compact"
        description={refunds.tableDescription(statusSummary.loaded)}
        emptyState={<div className="py-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">{hasActiveFilters ? refunds.emptyFiltered : refunds.empty}</div>}
        footer={(
          <SdkworkPaymentListPaginationControls
            busy={props.busy ?? false}
            label={messages.actions.loadMore}
            onLoadMore={props.onLoadMore}
            pageInfo={props.pageInfo}
            summary={props.pageInfo?.totalItems ? messages.table.paginationSummary(props.refunds.length, props.pageInfo.totalItems) : undefined}
          />
        )}
        getRowId={(refund) => refund.id}
        loading={props.busy && props.refunds.length === 0}
        loadingLabel={messages.table.loading}
        onRowClick={setViewing}
        rowActions={(refund) => (
          <div className="flex items-center gap-1">
            <Button
              aria-label={`${messages.operations.actions.view}: ${refund.refundNo}`}
              className={refund.status === "failed" && props.canRetry ? "hidden sm:inline-flex" : undefined}
              size="icon"
              title={refunds.availability.viewDetails}
              type="button"
              variant="ghost"
              onClick={() => setViewing(refund)}
            >
              <Eye aria-hidden="true" className="h-4 w-4" />
            </Button>
            {refund.status === "failed" && props.canRetry ? (
              <Button
                aria-label={`${messages.operations.actions.retryRefund}: ${refund.refundNo}`}
                size="icon"
                title={messages.operations.availability.retryRefund}
                type="button"
                variant="ghost"
                onClick={() => {
                  setRetryConfirmation("");
                  setRetrying(refund);
                }}
              >
                <RotateCcw aria-hidden="true" className="h-4 w-4" />
              </Button>
            ) : null}
          </div>
        )}
        rowActionsLabel={refunds.fields.actions}
        rows={Array.from(props.refunds)}
        slotProps={{
          table: {
            className: "[&_td:last-child]:hidden [&_th:last-child]:hidden sm:[&_td:last-child]:table-cell sm:[&_th:last-child]:table-cell",
          },
        }}
        stickyHeader
        title={messages.workspace.tabs.refunds}
      />

      <Drawer open={Boolean(viewing)} onOpenChange={(open) => { if (!open) setViewing(undefined); }}>
        <DrawerContent size="lg">
          {viewing ? (
            <RefundDetail
              refund={viewing}
              canRetry={props.canRetry}
              busy={props.busy}
              onClose={() => setViewing(undefined)}
              onRetry={() => {
                setViewing(undefined);
                setRetryConfirmation("");
                setRetrying(viewing);
              }}
            />
          ) : null}
        </DrawerContent>
      </Drawer>

      <Dialog open={Boolean(retrying)} onOpenChange={(open) => { if (!open && !props.busy) setRetrying(undefined); }}>
        <DialogContent>
          <DialogHeader><DialogTitle>{refunds.confirmationTitle}</DialogTitle></DialogHeader>
          {retrying ? (
            <div className="space-y-4">
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">{refunds.confirmationDescription(retrying.refundNo)}</p>
              <AdminFieldLabel label={refunds.form.retryConfirmation} htmlFor="refund-retry-confirmation" required>
                <Input id="refund-retry-confirmation" value={retryConfirmation} onChange={(event) => setRetryConfirmation(event.target.value)} placeholder={retrying.refundNo} autoComplete="off" />
              </AdminFieldLabel>
              <div className="flex justify-end gap-2">
                <Button type="button" variant="ghost" disabled={props.busy} onClick={() => setRetrying(undefined)}>{messages.operations.actions.cancel}</Button>
                <Button type="button" disabled={props.busy || retryConfirmation.trim() !== retrying.refundNo} onClick={() => void handleRetry()}>
                  <RotateCcw aria-hidden="true" className="mr-2 h-4 w-4" />
                  {messages.operations.actions.retryRefund}
                </Button>
              </div>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface RefundDetailProps {
  refund: RefundView;
  busy?: boolean;
  canRetry: boolean;
  onClose(): void;
  onRetry(): void;
}

function RefundDetail({ refund, busy, canRetry, onClose, onRetry }: RefundDetailProps) {
  const messages = usePaymentRecordsMessages();
  const refunds = messages.operations.refunds;

  return (
    <>
      <DrawerHeader>
        <div className="flex flex-wrap items-center gap-2">
          <DrawerTitle className="font-mono">{refund.refundNo}</DrawerTitle>
          <Badge variant={REFUND_BADGE_VARIANT[refund.status]}>{refunds.status[refund.status]}</Badge>
        </div>
        <DrawerDescription>{refunds.detailDescription(refunds.reason[refund.reasonCode ?? "other"])}</DrawerDescription>
      </DrawerHeader>
      <DrawerBody className="space-y-6">
        <DescriptionList columns={2}>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.amount}</DescriptionTerm>
            <DescriptionDetails>{formatAdminAmount(refund.amount, refund.currencyCode)}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.reason}</DescriptionTerm>
            <DescriptionDetails>{refunds.reason[refund.reasonCode ?? "other"]}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.payment}</DescriptionTerm>
            <DescriptionDetails mono>{refund.paymentIntentId}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.paymentAttempt}</DescriptionTerm>
            <DescriptionDetails mono>{refund.paymentAttemptId}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.order}</DescriptionTerm>
            <DescriptionDetails mono>{refund.orderId}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.providerAccount}</DescriptionTerm>
            <DescriptionDetails mono>{refund.providerAccountId ?? "--"}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.requestedBy}</DescriptionTerm>
            <DescriptionDetails>{refund.requestedBy ? `${refund.requestedByType}: ${refund.requestedBy}` : refund.requestedByType}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.created}</DescriptionTerm>
            <DescriptionDetails>{formatAdminTimestamp(refund.createdAt)}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>{refunds.fields.updated}</DescriptionTerm>
            <DescriptionDetails>{formatAdminTimestamp(refund.updatedAt)}</DescriptionDetails>
          </DescriptionItem>
        </DescriptionList>

        <div className="flex items-start gap-3 border-l-2 border-[color-mix(in_srgb,var(--sdk-color-state-info)_38%,transparent)] bg-[color-mix(in_srgb,var(--sdk-color-state-info)_10%,transparent)] px-3 py-2 text-sm text-[var(--sdk-color-text-secondary)]">
          <ShieldCheck aria-hidden="true" className="mt-0.5 h-4 w-4 shrink-0" />
          <span>{refunds.createDescription}</span>
        </div>
      </DrawerBody>
      <DrawerFooter>
        <Button type="button" size="sm" variant="ghost" onClick={onClose}>{messages.actions.close}</Button>
        {refund.status === "failed" && canRetry ? (
          <Button type="button" size="sm" disabled={busy} onClick={onRetry}>
            <RotateCcw aria-hidden="true" className="mr-2 h-4 w-4" />
            {messages.operations.actions.retryRefund}
          </Button>
        ) : null}
      </DrawerFooter>
    </>
  );
}

export interface RefundCreateDialogProps {
  open: boolean;
  intents: readonly PaymentIntentView[];
  initialIntent?: PaymentIntentView;
  busy?: boolean;
  onOpenChange(open: boolean): void;
  onSubmit(draft: CreateRefundDraft): Promise<void> | void;
}

export function RefundCreateDialog(props: RefundCreateDialogProps) {
  const messages = usePaymentRecordsMessages();
  const refunds = messages.operations.refunds;
  const succeededIntents = React.useMemo(
    () => props.intents.filter((intent) => ["succeeded", "refunding", "refunded"].includes(intent.status)),
    [props.intents],
  );
  const [paymentIntentId, setPaymentIntentId] = React.useState("");
  const [amount, setAmount] = React.useState("");
  const [reasonCode, setReasonCode] = React.useState<RefundReasonCode>("customer_request");
  const [confirmation, setConfirmation] = React.useState("");
  const [error, setError] = React.useState<string>();

  React.useEffect(() => {
    if (props.open) {
      setPaymentIntentId(props.initialIntent?.id ?? "");
      setAmount("");
      setReasonCode("customer_request");
      setConfirmation("");
      setError(undefined);
    }
  }, [props.initialIntent, props.open]);

  const selectedIntent = succeededIntents.find((intent) => intent.id === paymentIntentId);
  const amountError = React.useMemo(() => {
    const normalized = amount.trim();
    if (!normalized) {
      return undefined;
    }
    if (!/^\d+(\.\d{1,2})?$/.test(normalized) || Number(normalized) <= 0) {
      return messages.operations.validation.refundAmountInvalid;
    }
    if (selectedIntent && Number(normalized) > Number(selectedIntent.amount)) {
      return messages.operations.validation.refundAmountExceedsPayment;
    }
    return undefined;
  }, [amount, messages.operations.validation, selectedIntent]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!selectedIntent) {
      setError(messages.operations.validation.refundPaymentRequired);
      return;
    }
    if (confirmation.trim() !== selectedIntent.paymentIntentNo) {
      setError(messages.operations.validation.refundConfirmationRequired);
      return;
    }
    if (amountError) {
      setError(amountError);
      return;
    }
    setError(undefined);
    try {
      await props.onSubmit({
        paymentIntentId: selectedIntent.id,
        ...(amount.trim() ? { amount: amount.trim() } : {}),
        reasonCode,
        confirmPaymentIntentNo: confirmation.trim(),
      });
      props.onOpenChange(false);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : messages.operations.validation.createRefundFailed);
    }
  }

  return (
    <Dialog open={props.open} onOpenChange={(open) => { if (open || !props.busy) props.onOpenChange(open); }}>
      <DialogContent className="max-h-[calc(100dvh-2rem)] overflow-y-auto sm:max-w-xl">
        <DialogHeader><DialogTitle>{messages.operations.actions.newRefund}</DialogTitle></DialogHeader>
        <form className="space-y-4" onSubmit={handleSubmit}>
          <div className="flex gap-3 border-l-2 border-[var(--sdk-color-border-warning)] bg-[var(--sdk-color-bg-warning-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-secondary)]">
            <ShieldCheck aria-hidden="true" className="mt-0.5 h-4 w-4 shrink-0" />
            <span>{refunds.createDescription}</span>
          </div>
          <AdminFieldLabel label={refunds.form.payment} htmlFor="refund-create-payment" required>
            <Select disabled={props.busy || succeededIntents.length === 0} value={paymentIntentId} onValueChange={(value) => { setPaymentIntentId(value); setAmount(""); setConfirmation(""); setError(undefined); }}>
              <SelectTrigger id="refund-create-payment"><SelectValue placeholder={refunds.form.paymentPlaceholder} /></SelectTrigger>
              <SelectContent>
                {succeededIntents.map((intent) => (
                  <SelectItem key={intent.id} value={intent.id}>
                    {intent.paymentIntentNo} · {formatAdminAmount(intent.amount, intent.currencyCode)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </AdminFieldLabel>
          {succeededIntents.length === 0 ? (
            <div className="border-l-2 border-[var(--sdk-color-border-warning)] bg-[var(--sdk-color-bg-warning-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-secondary)]">
              {refunds.form.noEligiblePayments}
            </div>
          ) : null}
          {selectedIntent ? (
            <DescriptionList className="grid-cols-2" columns={2}>
              <DescriptionItem>
                <DescriptionTerm>{refunds.fields.payment}</DescriptionTerm>
                <DescriptionDetails className="break-all text-xs sm:text-sm" mono>{selectedIntent.paymentIntentNo}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.table.amount}</DescriptionTerm>
                <DescriptionDetails>{formatAdminAmount(selectedIntent.amount, selectedIntent.currencyCode)}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{refunds.fields.order}</DescriptionTerm>
                <DescriptionDetails className="break-all text-xs sm:text-sm" mono>{selectedIntent.orderId}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.provider}</DescriptionTerm>
                <DescriptionDetails>{selectedIntent.providerCode}</DescriptionDetails>
              </DescriptionItem>
            </DescriptionList>
          ) : null}
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
            <AdminFieldLabel label={refunds.form.amount} htmlFor="refund-create-amount">
              <Input aria-invalid={Boolean(amountError)} id="refund-create-amount" disabled={props.busy || !selectedIntent} inputMode="decimal" value={amount} onChange={(event) => { setAmount(event.target.value); setError(undefined); }} placeholder={selectedIntent?.amount ?? "0.00"} />
              <span className="text-xs font-normal text-[var(--sdk-color-text-muted)]">{refunds.form.amountHint}</span>
              {amountError ? <span className="text-xs font-normal text-[var(--sdk-color-text-error)]">{amountError}</span> : null}
            </AdminFieldLabel>
            <AdminFieldLabel label={refunds.form.reason} htmlFor="refund-create-reason" required>
              <Select disabled={props.busy || !selectedIntent} value={reasonCode} onValueChange={(value) => setReasonCode(value as RefundReasonCode)}>
                <SelectTrigger id="refund-create-reason"><SelectValue /></SelectTrigger>
                <SelectContent>
                  {REFUND_REASON_VALUES.map((value) => <SelectItem key={value} value={value}>{refunds.reason[value]}</SelectItem>)}
                </SelectContent>
              </Select>
            </AdminFieldLabel>
          </div>
          {selectedIntent ? (
            <AdminFieldLabel label={refunds.form.confirmation} htmlFor="refund-create-confirmation" required>
              <Input id="refund-create-confirmation" disabled={props.busy} value={confirmation} onChange={(event) => setConfirmation(event.target.value)} placeholder={selectedIntent.paymentIntentNo} autoComplete="off" />
              <span className="text-xs font-normal text-[var(--sdk-color-text-muted)]">{refunds.form.confirmationHint}</span>
            </AdminFieldLabel>
          ) : null}
          {error ? <div role="alert" className="border-l-2 border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-error)]">{error}</div> : null}
          <div className="flex justify-end gap-2">
            <Button type="button" variant="ghost" onClick={() => props.onOpenChange(false)} disabled={props.busy}>{messages.operations.actions.cancel}</Button>
            <Button type="submit" disabled={props.busy || !selectedIntent || Boolean(amountError) || confirmation.trim() !== selectedIntent.paymentIntentNo}>
              {messages.operations.actions.createRefund}
            </Button>
          </div>
        </form>
      </DialogContent>
    </Dialog>
  );
}
