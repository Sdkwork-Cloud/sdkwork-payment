import * as React from "react";
import {
  CalendarDays,
  Eye,
  RefreshCw,
  Search,
  SlidersHorizontal,
  X,
} from "lucide-react";
import {
  Badge,
  Button,
  DataTable,
  FilterBar,
  FilterBarActions,
  FilterBarSection,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  type DataTableColumn,
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  ADMIN_PROVIDER_FORM_OPTIONS,
  formatAdminAmount,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import { usePaymentRecordsMessages } from "../i18n";
import type {
  PaymentIntentDetail,
  PaymentIntentListFilter,
  PaymentIntentView,
  PaymentProviderCode,
  PaymentStatus,
} from "../types/monitor-admin-types";
import { PaymentRecordDetailDrawer } from "./PaymentRecordDetailDrawer";
import { PaymentRecordsOverview } from "./PaymentRecordsOverview";
import {
  formatPaymentProvider,
  PAYMENT_STATUS_BADGE_VARIANT,
} from "./payment-record-presentation";

export interface IntentMonitorProps {
  intents: readonly PaymentIntentView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  selectedIntent?: PaymentIntentDetail;
  onApplyFilter(filter: PaymentIntentListFilter): Promise<void> | void;
  onLoadMore(): void;
  onRefresh(): Promise<void> | void;
  onSelect(intent: PaymentIntentView): Promise<void> | void;
}

type StatusFilter = PaymentStatus | "all";
type ProviderFilter = PaymentProviderCode | "all";

const FILTERABLE_STATUS_VALUES: readonly PaymentStatus[] = [
  "created",
  "pending",
  "processing",
  "succeeded",
  "failed",
  "canceled",
  "closed",
];

function toDateTimeLocalValue(date: Date): string {
  const pad = (value: number) => String(value).padStart(2, "0");
  return [
    date.getFullYear(),
    "-",
    pad(date.getMonth() + 1),
    "-",
    pad(date.getDate()),
    "T",
    pad(date.getHours()),
    ":",
    pad(date.getMinutes()),
  ].join("");
}

export function IntentMonitor(props: IntentMonitorProps) {
  const messages = usePaymentRecordsMessages();
  const [status, setStatus] = React.useState<StatusFilter>("all");
  const [providerCode, setProviderCode] = React.useState<ProviderFilter>("all");
  const [ownerUserId, setOwnerUserId] = React.useState("");
  const [orderId, setOrderId] = React.useState("");
  const [currencyCode, setCurrencyCode] = React.useState("");
  const [createdAtFrom, setCreatedAtFrom] = React.useState("");
  const [createdAtTo, setCreatedAtTo] = React.useState("");
  const [q, setQ] = React.useState("");
  const [advancedOpen, setAdvancedOpen] = React.useState(false);
  const [error, setError] = React.useState<string>();
  const [selectedId, setSelectedId] = React.useState<string>();

  React.useEffect(() => {
    if (!props.selectedIntent) {
      setSelectedId(undefined);
    }
  }, [props.selectedIntent]);

  const activeFilterCount = [
    status !== "all",
    providerCode !== "all",
    Boolean(ownerUserId.trim()),
    Boolean(orderId.trim()),
    Boolean(currencyCode.trim()),
    Boolean(createdAtFrom),
    Boolean(createdAtTo),
    Boolean(q.trim()),
  ].filter(Boolean).length;

  async function applyFilter(filter: PaymentIntentListFilter) {
    setError(undefined);
    try {
      await props.onApplyFilter(filter);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : messages.validation.refreshFailed);
    }
  }

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    const from = createdAtFrom ? new Date(createdAtFrom) : undefined;
    const to = createdAtTo ? new Date(createdAtTo) : undefined;
    if (from && to && to <= from) {
      setError(messages.validation.invalidDateRange);
      return;
    }
    void applyFilter({
      ...(status !== "all" ? { status } : {}),
      ...(providerCode !== "all" ? { providerCode } : {}),
      ...(ownerUserId.trim() ? { ownerUserId: ownerUserId.trim() } : {}),
      ...(orderId.trim() ? { orderId: orderId.trim() } : {}),
      ...(currencyCode.trim() ? { currencyCode: currencyCode.trim().toUpperCase() } : {}),
      ...(from ? { createdAtFrom: from.toISOString() } : {}),
      ...(to ? { createdAtTo: to.toISOString() } : {}),
      ...(q.trim() ? { q: q.trim() } : {}),
    });
  }

  function handleResetFilter() {
    setStatus("all");
    setProviderCode("all");
    setOwnerUserId("");
    setOrderId("");
    setCurrencyCode("");
    setCreatedAtFrom("");
    setCreatedAtTo("");
    setQ("");
    setError(undefined);
    void applyFilter({});
  }

  function applyDatePreset(days: number) {
    const end = new Date();
    const start = new Date(end);
    start.setHours(0, 0, 0, 0);
    start.setDate(start.getDate() - Math.max(0, days - 1));
    setCreatedAtFrom(toDateTimeLocalValue(start));
    setCreatedAtTo(toDateTimeLocalValue(end));
  }

  async function handleRefresh() {
    setError(undefined);
    try {
      await props.onRefresh();
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : messages.validation.refreshFailed);
    }
  }

  async function handleSelect(intent: PaymentIntentView) {
    setSelectedId(intent.id);
    setError(undefined);
    try {
      await props.onSelect(intent);
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : messages.validation.loadDetailFailed);
    }
  }

  const columns: DataTableColumn<PaymentIntentView>[] = [
    {
      id: "payment",
      header: messages.table.payment,
      width: "22%",
      cell: (intent) => (
        <div className="min-w-[11rem] space-y-1">
          <div className="break-all font-mono text-sm font-semibold text-[var(--sdk-color-text-primary)]">
            {intent.paymentIntentNo}
          </div>
          <div className="text-xs text-[var(--sdk-color-text-muted)]">
            {intent.attempts?.length ?? 0} {messages.detail.attempts.toLowerCase()}
          </div>
        </div>
      ),
    },
    {
      id: "references",
      header: messages.table.references,
      width: "22%",
      headerProps: { className: "hidden lg:!table-cell" },
      cellProps: { className: "hidden lg:!table-cell" },
      cell: (intent) => (
        <div className="min-w-[10rem] space-y-1 text-xs">
          <div className="truncate font-mono text-[var(--sdk-color-text-primary)]" title={intent.orderId}>
            {intent.orderId || "--"}
          </div>
          <div className="truncate font-mono text-[var(--sdk-color-text-muted)]" title={intent.ownerUserId}>
            {intent.ownerUserId || "--"}
          </div>
        </div>
      ),
    },
    {
      id: "provider",
      header: messages.table.providerAndMethod,
      width: "18%",
      cell: (intent) => (
        <div className="min-w-[8rem] space-y-1">
          <span className="text-sm font-medium text-[var(--sdk-color-text-primary)]">
            {formatPaymentProvider(intent.providerCode)}
          </span>
          <div className="text-xs text-[var(--sdk-color-text-muted)]">{intent.paymentMethod || "--"}</div>
        </div>
      ),
    },
    {
      align: "right",
      id: "amount",
      header: messages.table.amount,
      width: "14%",
      cell: (intent) => (
        <div className="min-w-[7rem] text-right text-sm font-semibold tabular-nums text-[var(--sdk-color-text-primary)]">
          {formatAdminAmount(intent.amount, intent.currencyCode)}
        </div>
      ),
    },
    {
      id: "status",
      header: messages.table.status,
      width: "12%",
      cell: (intent) => (
        <Badge variant={PAYMENT_STATUS_BADGE_VARIANT[intent.status]}>
          {messages.status[intent.status]}
        </Badge>
      ),
    },
    {
      id: "createdAt",
      header: messages.table.createdAt,
      width: "12%",
      headerProps: { className: "hidden lg:!table-cell" },
      cellProps: { className: "hidden lg:!table-cell" },
      cell: (intent) => (
        <div className="min-w-[8rem] text-xs">
          <div className="text-[var(--sdk-color-text-primary)]">{formatAdminRelativeTime(intent.createdAt)}</div>
          <div className="mt-1 text-[var(--sdk-color-text-muted)]">{formatAdminTimestamp(intent.createdAt)}</div>
        </div>
      ),
    },
  ];

  return (
    <div className="space-y-4" data-slot="payment-intent-monitor">
      <PaymentRecordsOverview intents={props.intents} pageInfo={props.pageInfo} />

      <form onSubmit={handleApply}>
        <FilterBar
          description={messages.filters.description}
          summary={activeFilterCount > 0 ? messages.filters.activeSummary(activeFilterCount) : undefined}
          title={messages.filters.title}
        >
          <FilterBarSection className="min-w-full xl:min-w-0" grow>
            <AdminFieldLabel
              className="min-w-[16rem] flex-[2]"
              htmlFor="payment-record-filter-search"
              label={messages.filters.search}
            >
              <div className="relative">
                <Search aria-hidden="true" className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-[var(--sdk-color-text-muted)]" />
                <Input
                  className="pl-9"
                  id="payment-record-filter-search"
                  placeholder={messages.filters.searchPlaceholder}
                  value={q}
                  onChange={(event) => setQ(event.target.value)}
                />
              </div>
            </AdminFieldLabel>
            <AdminFieldLabel
              className="min-w-[11rem] flex-1"
              htmlFor="payment-record-filter-status"
              label={messages.filters.status}
            >
              <Select value={status} onValueChange={(value) => setStatus(value as StatusFilter)}>
                <SelectTrigger id="payment-record-filter-status">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{messages.filters.allStatuses}</SelectItem>
                  {FILTERABLE_STATUS_VALUES.map((value) => (
                    <SelectItem key={value} value={value}>{messages.status[value]}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </AdminFieldLabel>
            <AdminFieldLabel
              className="min-w-[11rem] flex-1"
              htmlFor="payment-record-filter-provider"
              label={messages.filters.provider}
            >
              <Select value={providerCode} onValueChange={(value) => setProviderCode(value as ProviderFilter)}>
                <SelectTrigger id="payment-record-filter-provider">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{messages.filters.allProviders}</SelectItem>
                  {ADMIN_PROVIDER_FORM_OPTIONS.map((option) => (
                    <SelectItem key={option.value} value={option.value}>{option.label}</SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </AdminFieldLabel>
          </FilterBarSection>

          <FilterBarActions>
            <Button
              type="button"
              variant="ghost"
              onClick={() => setAdvancedOpen((open) => !open)}
            >
              <SlidersHorizontal aria-hidden="true" className="mr-2 h-4 w-4" />
              {advancedOpen ? messages.actions.hideAdvanced : messages.actions.showAdvanced}
            </Button>
            <Button type="button" variant="ghost" disabled={props.busy || activeFilterCount === 0} onClick={handleResetFilter}>
              <X aria-hidden="true" className="mr-2 h-4 w-4" />
              {messages.actions.clearFilters}
            </Button>
            <Button type="submit" disabled={props.busy}>
              <Search aria-hidden="true" className="mr-2 h-4 w-4" />
              {messages.actions.applyFilters}
            </Button>
          </FilterBarActions>

          {advancedOpen ? (
            <div className="w-full space-y-3 border-t border-[var(--sdk-color-border-subtle)] pt-3">
              <p className="text-xs text-[var(--sdk-color-text-secondary)]">{messages.filters.advancedDescription}</p>
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:!grid-cols-5">
                <AdminFieldLabel htmlFor="payment-record-filter-order" label={messages.filters.orderIdentifier}>
                  <Input
                    id="payment-record-filter-order"
                    placeholder={messages.filters.orderPlaceholder}
                    value={orderId}
                    onChange={(event) => setOrderId(event.target.value)}
                  />
                </AdminFieldLabel>
                <AdminFieldLabel htmlFor="payment-record-filter-owner" label={messages.filters.ownerIdentifier}>
                  <Input
                    id="payment-record-filter-owner"
                    placeholder={messages.filters.ownerPlaceholder}
                    value={ownerUserId}
                    onChange={(event) => setOwnerUserId(event.target.value)}
                  />
                </AdminFieldLabel>
                <AdminFieldLabel htmlFor="payment-record-filter-currency" label={messages.filters.currency}>
                  <Input
                    id="payment-record-filter-currency"
                    maxLength={3}
                    placeholder={messages.filters.currencyPlaceholder}
                    value={currencyCode}
                    onChange={(event) => setCurrencyCode(event.target.value)}
                  />
                </AdminFieldLabel>
                <AdminFieldLabel htmlFor="payment-record-filter-from" label={messages.filters.createdFrom}>
                  <Input
                    id="payment-record-filter-from"
                    type="datetime-local"
                    value={createdAtFrom}
                    onChange={(event) => setCreatedAtFrom(event.target.value)}
                  />
                </AdminFieldLabel>
                <AdminFieldLabel htmlFor="payment-record-filter-to" label={messages.filters.createdTo}>
                  <Input
                    id="payment-record-filter-to"
                    type="datetime-local"
                    value={createdAtTo}
                    onChange={(event) => setCreatedAtTo(event.target.value)}
                  />
                </AdminFieldLabel>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <CalendarDays aria-hidden="true" className="h-4 w-4 text-[var(--sdk-color-text-muted)]" />
                <Button type="button" size="sm" variant="outline" onClick={() => applyDatePreset(1)}>{messages.filters.today}</Button>
                <Button type="button" size="sm" variant="outline" onClick={() => applyDatePreset(7)}>{messages.filters.last7Days}</Button>
                <Button type="button" size="sm" variant="outline" onClick={() => applyDatePreset(30)}>{messages.filters.last30Days}</Button>
              </div>
            </div>
          ) : null}
        </FilterBar>
      </form>

      {error ? (
        <div
          className="border-l-2 border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] px-3 py-2 text-sm text-[var(--sdk-color-text-error)]"
          role="alert"
        >
          {error}
        </div>
      ) : null}

      <DataTable
        columns={columns}
        density="compact"
        description={messages.table.resultDescription(props.intents.length)}
        emptyState={(
          <div className="space-y-3 py-6 text-center">
            <div>
              <h3 className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">{messages.empty.title}</h3>
              <p className="mt-1 text-sm text-[var(--sdk-color-text-secondary)]">{messages.empty.description}</p>
            </div>
            <Button type="button" size="sm" variant="outline" disabled={props.busy} onClick={handleResetFilter}>
              <X aria-hidden="true" className="mr-2 h-4 w-4" />
              {messages.actions.clearFilters}
            </Button>
          </div>
        )}
        footer={(
          <SdkworkPaymentListPaginationControls
            busy={props.busy ?? false}
            label={messages.actions.loadMore}
            onLoadMore={props.onLoadMore}
            pageInfo={props.pageInfo}
            summary={props.pageInfo?.totalItems
              ? messages.table.paginationSummary(props.intents.length, props.pageInfo.totalItems)
              : undefined}
          />
        )}
        getRowId={(intent) => intent.id}
        getRowProps={(intent) => ({
          className: selectedId === intent.id ? "bg-[var(--sdk-color-bg-subtle)]" : undefined,
        })}
        loading={props.busy && props.intents.length === 0}
        loadingLabel={messages.table.loading}
        rowActions={(intent) => (
          <Button
            aria-label={`${messages.actions.viewDetails}: ${intent.paymentIntentNo}`}
            size="icon"
            title={messages.actions.viewDetails}
            type="button"
            variant="ghost"
            onClick={() => void handleSelect(intent)}
          >
            <Eye aria-hidden="true" className="h-4 w-4" />
          </Button>
        )}
        rowActionsLabel=""
        rows={Array.from(props.intents)}
        stickyHeader
        title={messages.table.title}
        toolbar={(
          <Button
            aria-label={messages.actions.refresh}
            disabled={props.busy}
            size="icon"
            title={messages.actions.refresh}
            type="button"
            variant="outline"
            onClick={() => void handleRefresh()}
          >
            <RefreshCw aria-hidden="true" className={`h-4 w-4 ${props.busy ? "animate-spin" : ""}`} />
          </Button>
        )}
      />

      <PaymentRecordDetailDrawer
        detail={props.selectedIntent}
        open={Boolean(selectedId && props.selectedIntent)}
        onOpenChange={(open) => {
          if (!open) {
            setSelectedId(undefined);
          }
        }}
      />
    </div>
  );
}
