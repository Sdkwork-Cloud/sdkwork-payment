/**
 * Payment admin monitor workspace.
 *
 * Four-tab workspace for payment operations monitoring:
 *   1. Intents — list + filter + retrieve detail (the "why" of a payment)
 *   2. Attempts — list + filter (the "how" — PSP-facing execution records)
 *   3. Webhook Events — list + filter + replay (inbound event stream)
 *   4. Reconciliation Runs — list + filter + create (settlement matching)
 *
 * Mirrors industry PSP operations consoles (Stripe Dashboard → Payments /
 * Events / Reports, Adyen → Settlements, WeChat Pay → transaction monitoring).
 *
 * Uses an external store subscription pattern (subscribe/getState) so the host
 * app can wire it into React's useSyncExternalStore if needed.
 */

import * as React from "react";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@sdkwork/ui-pc-react";
import {
  PaymentAdminTabsContent,
  PaymentAdminTabsList,
  PaymentAdminTabsTrigger,
  PaymentAdminWorkspace,
} from "@sdkwork/payment-pc-admin-core";
import { usePaymentRecordsMessages } from "../i18n";
import { AttemptMonitor } from "../components/AttemptMonitor";
import { IntentMonitor } from "../components/IntentMonitor";
import { ReconciliationMonitor } from "../components/ReconciliationMonitor";
import { RefundCreateDialog, RefundMonitor } from "../components/RefundMonitor";
import { WebhookEventMonitor } from "../components/WebhookEventMonitor";
import type {
  CreateReconciliationRunDraft,
  CreateRefundDraft,
  PaymentAttemptListFilter,
  PaymentIntentListFilter,
  PaymentIntentView,
  PaymentMonitorAdminController,
  PaymentMonitorAdminState,
  PaymentWebhookEventListFilter,
  ReconciliationRunListFilter,
  RefundListFilter,
} from "../types/monitor-admin-types";

export interface PaymentMonitorAdminWorkspaceProps {
  controller: PaymentMonitorAdminController;
  capabilities: PaymentMonitorAdminCapabilities;
  section?: PaymentMonitorAdminSection;
  title?: string;
  description?: string;
}

export interface PaymentMonitorAdminCapabilities {
  canCreateRefund: boolean;
  canCreateReconciliationRun: boolean;
  canReplayWebhookEvent: boolean;
  canRetryRefund: boolean;
}

export type PaymentMonitorAdminSection =
  | "intents"
  | "attempts"
  | "refunds"
  | "webhooks"
  | "reconciliation";

export function PaymentMonitorAdminWorkspace(
  props: PaymentMonitorAdminWorkspaceProps,
) {
  const { controller } = props;
  const messages = usePaymentRecordsMessages();
  const [state, setState] = React.useState<PaymentMonitorAdminState>(() =>
    controller.getState(),
  );
  const [tab, setTab] = React.useState<PaymentMonitorAdminSection>("intents");
  const [refundDialogOpen, setRefundDialogOpen] = React.useState(false);
  const [refundIntent, setRefundIntent] = React.useState<PaymentIntentView>();
  const activeSection = props.section ?? tab;

  React.useEffect(() => {
    return controller.subscribe(() => {
      setState(controller.getState());
    });
  }, [controller]);

  React.useEffect(() => {
    void controller.load().then(setState).catch(() => {
      // error already surfaced via controller state.lastError
    });
  }, [controller]);

  const busy = state.status === "loading" || state.status === "saving";

  async function handleApplyIntentFilter(filter: PaymentIntentListFilter) {
    await controller.applyIntentFilter(filter);
  }

  async function handleApplyAttemptFilter(filter: PaymentAttemptListFilter) {
    await controller.applyAttemptFilter(filter);
  }

  async function handleApplyWebhookFilter(filter: PaymentWebhookEventListFilter) {
    await controller.applyWebhookEventFilter(filter);
  }

  async function handleApplyReconciliationFilter(filter: ReconciliationRunListFilter) {
    await controller.applyReconciliationRunFilter(filter);
  }

  async function handleSelectIntent(intent: PaymentIntentView) {
    await controller.selectIntent(intent.id);
  }

  async function handleReplay(eventId: string) {
    await controller.replayWebhookEvent(eventId);
  }

  async function handleCreateReconciliationRun(draft: CreateReconciliationRunDraft) {
    await controller.createReconciliationRun(draft);
  }

  async function handleApplyRefundFilter(filter: RefundListFilter) {
    await controller.applyRefundFilter(filter);
  }

  async function handleCreateRefund(draft: CreateRefundDraft) {
    await controller.createRefund(draft);
  }

  function handleStartRefund(intent?: PaymentIntentView) {
    setRefundIntent(intent);
    setRefundDialogOpen(true);
  }

  return (
    <PaymentAdminWorkspace
      data-slot="payment-monitor-admin-workspace"
      description={props.description ?? messages.workspace.description}
      error={state.lastError}
      title={props.title ?? messages.workspace.title}
    >
        <Tabs
          value={activeSection}
          onValueChange={(value) => {
            if (!props.section) {
              setTab(value as PaymentMonitorAdminSection);
            }
          }}
        >
          {!props.section ? (
            <PaymentAdminTabsList
              aria-label={messages.workspace.tabsLabel}
              className="grid h-9 grid-cols-5 overflow-visible sm:!flex sm:!overflow-x-auto"
            >
              <PaymentAdminTabsTrigger
                className="h-9 min-w-0 shrink whitespace-nowrap px-0.5 leading-tight sm:!min-w-fit sm:!shrink-0 sm:!px-3"
                value="intents"
              >
                {messages.workspace.tabs.paymentRecords}
              </PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger
                className="h-9 min-w-0 shrink whitespace-nowrap px-0.5 leading-tight sm:!min-w-fit sm:!shrink-0 sm:!px-3"
                value="attempts"
              >
                {messages.workspace.tabs.attempts}
              </PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger
                className="h-9 min-w-0 shrink whitespace-nowrap px-0.5 leading-tight sm:!min-w-fit sm:!shrink-0 sm:!px-3"
                value="refunds"
              >
                {messages.workspace.tabs.refunds}
              </PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger
                className="h-9 min-w-0 shrink whitespace-nowrap px-0.5 leading-tight sm:!min-w-fit sm:!shrink-0 sm:!px-3"
                value="webhooks"
              >
                {messages.workspace.tabs.webhooks}
              </PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger
                className="h-9 min-w-0 shrink whitespace-nowrap px-0.5 leading-tight sm:!min-w-fit sm:!shrink-0 sm:!px-3"
                value="reconciliation"
              >
                {messages.workspace.tabs.reconciliation}
              </PaymentAdminTabsTrigger>
            </PaymentAdminTabsList>
          ) : null}

          <PaymentAdminTabsContent value="intents">
            <IntentMonitor
              intents={state.intents}
              pageInfo={state.listPageInfo?.intents}
              busy={busy}
              selectedIntent={state.selectedIntentDetail}
              canCreateRefund={props.capabilities.canCreateRefund}
              onApplyFilter={handleApplyIntentFilter}
              onLoadMore={() => void controller.loadMoreIntents()}
              onRefresh={() => controller.refreshIntents().then(() => undefined)}
              onSelect={handleSelectIntent}
              onRefund={handleStartRefund}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="attempts">
            <AttemptMonitor
              attempts={state.attempts}
              pageInfo={state.listPageInfo?.attempts}
              busy={busy}
              onApplyFilter={handleApplyAttemptFilter}
              onLoadMore={() => void controller.loadMoreAttempts()}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="refunds">
            <RefundMonitor
              refunds={state.refunds}
              pageInfo={state.listPageInfo?.refunds}
              busy={busy}
              canCreate={props.capabilities.canCreateRefund}
              canRetry={props.capabilities.canRetryRefund}
              onApplyFilter={handleApplyRefundFilter}
              onLoadMore={() => void controller.loadMoreRefunds()}
              onStartCreate={() => handleStartRefund()}
              onRetry={(refundId, confirmRefundNo) => controller.retryRefund(refundId, confirmRefundNo)}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="webhooks">
            <WebhookEventMonitor
              events={state.webhookEvents}
              pageInfo={state.listPageInfo?.webhookEvents}
              busy={busy}
              canReplay={props.capabilities.canReplayWebhookEvent}
              lastReplayResult={state.lastReplayResult}
              onApplyFilter={handleApplyWebhookFilter}
              onLoadMore={() => void controller.loadMoreWebhookEvents()}
              onReplay={handleReplay}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="reconciliation">
            <ReconciliationMonitor
              runs={state.reconciliationRuns}
              pageInfo={state.listPageInfo?.reconciliationRuns}
              busy={busy}
              canCreate={props.capabilities.canCreateReconciliationRun}
              // Provider account dropdown data source: read from controller state (empty array when not yet wired up)
              providerAccounts={state.providerAccounts ?? []}
              onApplyFilter={handleApplyReconciliationFilter}
              onLoadMore={() => void controller.loadMoreReconciliationRuns()}
              onCreate={handleCreateReconciliationRun}
            />
          </PaymentAdminTabsContent>
        </Tabs>
        <RefundCreateDialog
          open={props.capabilities.canCreateRefund && refundDialogOpen}
          intents={state.intents}
          initialIntent={refundIntent}
          busy={busy}
          onOpenChange={(open) => {
            setRefundDialogOpen(open);
            if (!open) setRefundIntent(undefined);
          }}
          onSubmit={handleCreateRefund}
        />
    </PaymentAdminWorkspace>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentMonitorAdminTabs, TabsList, TabsTrigger, TabsContent };
