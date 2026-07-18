/**
 * Webhook event monitor.
 *
 * Lists webhook events with filter (status / providerCode / eventType / q) and
 * replay action. Mirrors PSP webhook event consoles (Stripe Dashboard →
 * Developers → Events, Alipay merchant platform → webhook log).
 *
 * API matrix: list + replay. Replay is capped at 5 retries (WEBHOOK_STORED_
 * ADMIN_WEBHOOK_REPLAY_MAX_RETRIES); events beyond that are marked "dead" and the replay
 * button is disabled.
 *
 * Each row shows a signature-status badge (valid/invalid/unverified) alongside
 * the processing status — mirroring Stripe Dashboard's webhook event detail
 * "Signature" indicator. A "View" button opens a right-side Drawer with the
 * raw payload (JSON viewer) and request headers, so operators can inspect
 * the exact PSP notification that triggered the event.
 */

import * as React from "react";
import {
  Badge,
  Button,
  DescriptionDetails,
  DescriptionItem,
  DescriptionList,
  DescriptionTerm,
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
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  ADMIN_PROVIDER_FILTER_OPTIONS,
  ADMIN_PROVIDER_LABEL,
  ADMIN_WEBHOOK_REPLAY_MAX_RETRIES,
  ConfirmDialog,
  formatAdminRelativeTime,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentProviderCode,
  PaymentWebhookEventListFilter,
  PaymentWebhookEventView,
  WebhookEventStatus,
  WebhookReplayResult,
  WebhookSignatureStatus,
} from "../types/monitor-admin-types";

export interface WebhookEventMonitorProps {
  events: readonly PaymentWebhookEventView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  canReplay: boolean;
  lastReplayResult?: WebhookReplayResult;
  onApplyFilter(filter: PaymentWebhookEventListFilter): Promise<void> | void;
  onLoadMore(): void;
  onReplay(eventId: string): Promise<void> | void;
}

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: WebhookEventStatus | "" }> = [
  { label: "All statuses", value: "" },
  { label: "Queued", value: "queued" },
  { label: "Processing", value: "processing" },
  { label: "Processed", value: "processed" },
  { label: "Failed", value: "failed" },
  { label: "Dead", value: "dead" },
];

const STATUS_VARIANT: Record<WebhookEventStatus, "default" | "success" | "warning" | "danger" | "secondary"> = {
  queued: "secondary",
  processing: "warning",
  processed: "success",
  failed: "danger",
  dead: "danger",
};

// Signature status badge — aligns with the Stripe Dashboard "Signature verified" indicator
const SIGNATURE_VARIANT: Record<WebhookSignatureStatus, "success" | "danger" | "secondary" | "warning"> = {
  valid: "success",
  invalid: "danger",
  unverified: "secondary",
  unknown: "warning",
};

const SIGNATURE_LABEL: Record<WebhookSignatureStatus, string> = {
  valid: "Signature verified",
  invalid: "Signature invalid",
  unverified: "Signature unchecked",
  unknown: "Signature unknown",
};

function formatPayloadJson(payload: unknown): string {
  try {
    return JSON.stringify(payload, null, 2);
  } catch {
    return String(payload);
  }
}

export function WebhookEventMonitor(props: WebhookEventMonitorProps) {
  const [status, setStatus] = React.useState<string>("");
  const [providerCode, setProviderCode] = React.useState<string>("");
  const [eventType, setEventType] = React.useState("");
  const [q, setQ] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();
  const [pendingReplay, setPendingReplay] = React.useState<PaymentWebhookEventView | null>(null);
  const [viewingEvent, setViewingEvent] = React.useState<PaymentWebhookEventView | null>(null);

  function handleApply(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    const filter: PaymentWebhookEventListFilter = {
      ...(status ? { status: status as WebhookEventStatus } : {}),
      ...(providerCode ? { providerCode: providerCode as PaymentProviderCode } : {}),
      ...(eventType.trim() ? { eventType: eventType.trim() } : {}),
      ...(q.trim() ? { q: q.trim() } : {}),
    };
    Promise.resolve(props.onApplyFilter(filter)).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to apply filter.");
    });
  }

  function handleResetFilter() {
    setStatus("");
    setProviderCode("");
    setEventType("");
    setQ("");
    setError(undefined);
    Promise.resolve(props.onApplyFilter({})).catch((err) => {
      setError(err instanceof Error ? err.message : "Failed to clear filters.");
    });
  }

  async function handleReplay() {
    if (!pendingReplay) return;
    const eventId = pendingReplay.eventId ?? pendingReplay.id;
    setError(undefined);
    try {
      await props.onReplay(eventId);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to replay webhook event.");
    }
    setPendingReplay(null);
  }

  // Resolve the latest viewingEvent reference from the list (data may update after replay/reload)
  const viewingEventResolved = viewingEvent
    ? props.events.find((e) => e.id === viewingEvent.id) ?? viewingEvent
    : null;

  return (
    <div className="space-y-4" data-slot="payment-webhook-event-monitor">
      <form
        className="grid grid-cols-1 gap-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3 sm:grid-cols-2 lg:grid-cols-4"
        onSubmit={handleApply}
      >
        <AdminFieldLabel label="Status" htmlFor="webhook-filter-status">
          <Select value={status} onValueChange={setStatus}>
            <SelectTrigger id="webhook-filter-status">
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
        <AdminFieldLabel label="Provider" htmlFor="webhook-filter-provider">
          <Select value={providerCode} onValueChange={setProviderCode}>
            <SelectTrigger id="webhook-filter-provider">
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
        <AdminFieldLabel label="Event type" htmlFor="webhook-filter-event-type">
          <Input
            id="webhook-filter-event-type"
            value={eventType}
            onChange={(event) => setEventType(event.target.value)}
            placeholder="e.g., payment_intent.succeeded"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Search" htmlFor="webhook-filter-q">
          <Input
            id="webhook-filter-q"
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

      {props.lastReplayResult ? (
        <div
          role="status"
          className={
            "rounded-md border p-3 text-sm " +
            (props.lastReplayResult.ok
              ? "border-[var(--sdk-color-border-success)] bg-[var(--sdk-color-bg-success-subtle)] text-[var(--sdk-color-text-success)]"
              : "border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] text-[var(--sdk-color-text-error)]")
          }
        >
          Replay {props.lastReplayResult.ok ? "accepted" : "rejected"} for event{" "}
          <span className="font-mono">{props.lastReplayResult.eventId}</span> at{" "}
          {formatAdminTimestamp(props.lastReplayResult.replayedAt)}
          {props.lastReplayResult.diagnostic ? ` — ${props.lastReplayResult.diagnostic}` : ""}
        </div>
      ) : null}

      {props.events.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No webhook events found. Adjust the filter or wait for incoming events.
          <div className="mt-3">
            <Button type="button" variant="ghost" size="sm" onClick={handleResetFilter} disabled={props.busy}>
              Clear filters
            </Button>
          </div>
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.events.map((event) => {
            const replayDisabled =
              event.status === "dead" || event.retries >= ADMIN_WEBHOOK_REPLAY_MAX_RETRIES;
            const signatureStatus = event.signatureStatus ?? "unverified";
            return (
              <li key={event.id} className="flex flex-col gap-2 p-4 sm:flex-row sm:items-center sm:justify-between">
                <div className="min-w-0 flex-1">
                  <div className="flex flex-wrap items-center gap-2">
                    <button
                      type="button"
                      className="font-mono text-sm font-medium text-[var(--sdk-color-text)] underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--sdk-color-border-focus)] focus-visible:ring-offset-2"
                      onClick={() => setViewingEvent(event)}
                      title="View webhook event payload and details"
                    >
                      {event.eventType}
                    </button>
                    <Badge variant="outline" className="font-mono">
                      {ADMIN_PROVIDER_LABEL[event.providerCode]}
                    </Badge>
                    <Badge variant={STATUS_VARIANT[event.status]}>{event.status}</Badge>
                    <Badge
                      variant={SIGNATURE_VARIANT[signatureStatus]}
                      title={SIGNATURE_LABEL[signatureStatus]}
                    >
                      {SIGNATURE_LABEL[signatureStatus]}
                    </Badge>
                    <Badge variant="secondary">retries: {event.retries}/{ADMIN_WEBHOOK_REPLAY_MAX_RETRIES}</Badge>
                  </div>
                  <dl className="mt-2 grid grid-cols-2 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-3">
                    <div>
                      <dt className="inline">Event ID:</dt>{" "}
                      <dd className="inline font-mono">{event.eventId ?? "—"}</dd>
                    </div>
                    <div>
                      <dt className="inline">Received:</dt> <dd className="inline">{formatAdminRelativeTime(event.receivedAt)}</dd>
                    </div>
                    <div>
                      <dt className="inline">Processed:</dt>{" "}
                      <dd className="inline">{event.processedAt ? formatAdminTimestamp(event.processedAt) : "—"}</dd>
                    </div>
                  </dl>
                  {event.lastError ? (
                    <p className="mt-1 text-xs text-[var(--sdk-color-text-error)]">
                      Last error: {event.lastError}
                    </p>
                  ) : null}
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setViewingEvent(event)}
                    disabled={props.busy}
                    title={props.busy ? "Cannot open details while another operation is in progress" : "View webhook event payload and details"}
                  >
                    View
                  </Button>
                  {props.canReplay ? (
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      onClick={() => setPendingReplay(event)}
                      disabled={props.busy || replayDisabled}
                      title={
                        replayDisabled
                          ? "Replay disabled: max retries reached or event marked dead"
                          : "Replay this webhook event"
                      }
                    >
                      Replay
                    </Button>
                  ) : null}
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
      />

      {/* Webhook event detail Drawer — payload viewer + headers, mirroring Stripe Dashboard event detail */}
      <Drawer
        open={viewingEventResolved !== null}
        onOpenChange={(open) => {
          if (!open) setViewingEvent(null);
        }}
      >
        <DrawerContent size="lg">
          {viewingEventResolved ? (
            <WebhookEventDetail
              event={viewingEventResolved}
              busy={props.busy}
              canReplay={props.canReplay}
              replayDisabled={
                viewingEventResolved.status === "dead" ||
                viewingEventResolved.retries >= ADMIN_WEBHOOK_REPLAY_MAX_RETRIES
              }
              onReplay={() => {
                setPendingReplay(viewingEventResolved);
                setViewingEvent(null);
              }}
            />
          ) : null}
        </DrawerContent>
      </Drawer>

      <ConfirmDialog
        open={props.canReplay && pendingReplay !== null}
        title="Replay webhook event?"
        description={
          pendingReplay
            ? `Replay ${pendingReplay.eventType} (${pendingReplay.eventId ?? pendingReplay.id})? This will re-send the webhook notification to all registered endpoints, which may trigger downstream side effects (e.g., duplicate order fulfillment).`
            : ""
        }
        confirmLabel="Replay"
        variant="warning"
        busy={props.busy}
        onConfirm={handleReplay}
        onOpenChange={(open) => {
          if (!open) setPendingReplay(null);
        }}
      />
    </div>
  );
}

// ---------------------------------------------------------------------------
// Webhook event detail (payload viewer)
// ---------------------------------------------------------------------------

interface WebhookEventDetailProps {
  event: PaymentWebhookEventView;
  busy?: boolean;
  canReplay: boolean;
  replayDisabled: boolean;
  onReplay(): void;
}

function WebhookEventDetail(props: WebhookEventDetailProps) {
  const { event } = props;
  const signatureStatus = event.signatureStatus ?? "unverified";
  const payloadJson = event.payload ? formatPayloadJson(event.payload) : null;
  const headerEntries = event.headers ? Object.entries(event.headers) : [];

  return (
    <>
      <DrawerHeader>
        <DrawerTitle className="font-mono">{event.eventType}</DrawerTitle>
        <DrawerDescription>
          Webhook event <span className="font-mono">{event.eventId ?? event.id}</span> from{" "}
          {ADMIN_PROVIDER_LABEL[event.providerCode]}
        </DrawerDescription>
      </DrawerHeader>
      <DrawerBody className="space-y-6">
        <DescriptionList columns={2}>
          <DescriptionItem>
            <DescriptionTerm>Event ID</DescriptionTerm>
            <DescriptionDetails mono>{event.eventId ?? "—"}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Provider</DescriptionTerm>
            <DescriptionDetails>{ADMIN_PROVIDER_LABEL[event.providerCode]}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Status</DescriptionTerm>
            <DescriptionDetails>
              <Badge variant={STATUS_VARIANT[event.status]}>{event.status}</Badge>
            </DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Signature</DescriptionTerm>
            <DescriptionDetails>
              <Badge variant={SIGNATURE_VARIANT[signatureStatus]}>{SIGNATURE_LABEL[signatureStatus]}</Badge>
            </DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Retries</DescriptionTerm>
            <DescriptionDetails>{event.retries} / {ADMIN_WEBHOOK_REPLAY_MAX_RETRIES}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Received</DescriptionTerm>
            <DescriptionDetails>{formatAdminTimestamp(event.receivedAt)}</DescriptionDetails>
          </DescriptionItem>
          <DescriptionItem>
            <DescriptionTerm>Processed</DescriptionTerm>
            <DescriptionDetails>{event.processedAt ? formatAdminTimestamp(event.processedAt) : "—"}</DescriptionDetails>
          </DescriptionItem>
        </DescriptionList>

        {event.lastError ? (
          <div
            role="alert"
            className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
          >
            <div className="font-medium">Last error</div>
            <pre className="mt-1 whitespace-pre-wrap break-all font-mono text-xs">{event.lastError}</pre>
          </div>
        ) : null}

        {payloadJson ? (
          <section className="space-y-2">
            <h4 className="text-xs font-medium uppercase tracking-[0.12em] text-[var(--sdk-color-text-muted)]">
              Payload
            </h4>
            <pre
              className="max-h-[24rem] overflow-auto rounded-md border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] p-3 font-mono text-xs leading-relaxed text-[var(--sdk-color-text-primary)]"
              data-slot="webhook-payload-viewer"
            >
              {payloadJson}
            </pre>
          </section>
        ) : (
          <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-4 text-center text-sm text-[var(--sdk-color-text-secondary)]">
            No payload captured for this event.
          </div>
        )}

        {headerEntries.length > 0 ? (
          <section className="space-y-2">
            <h4 className="text-xs font-medium uppercase tracking-[0.12em] text-[var(--sdk-color-text-muted)]">
              Request headers
            </h4>
            <dl className="grid grid-cols-1 gap-x-6 gap-y-1 rounded-md border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] p-3 sm:grid-cols-2">
              {headerEntries.map(([key, value]) => (
                <div key={key} className="flex flex-col gap-0.5">
                  <dt className="font-mono text-xs font-medium text-[var(--sdk-color-text-secondary)]">{key}</dt>
                  <dd className="font-mono text-xs text-[var(--sdk-color-text-primary)] break-all">{value}</dd>
                </div>
              ))}
            </dl>
          </section>
        ) : null}
      </DrawerBody>
      <DrawerFooter>
        {props.canReplay ? (
          <Button
            type="button"
            variant="ghost"
            size="sm"
            onClick={props.onReplay}
            disabled={props.busy || props.replayDisabled}
            title={
              props.replayDisabled
                ? "Replay disabled: max retries reached or event marked dead"
                : "Replay this webhook event"
            }
          >
            Replay
          </Button>
        ) : null}
      </DrawerFooter>
    </>
  );
}
