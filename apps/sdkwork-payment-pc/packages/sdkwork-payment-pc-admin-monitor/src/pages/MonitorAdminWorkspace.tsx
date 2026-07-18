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
  PaymentAdminI18nBoundary,
  PaymentAdminTabsContent,
  PaymentAdminTabsList,
  PaymentAdminTabsTrigger,
  PaymentAdminWorkspace,
} from "@sdkwork/payment-pc-admin-core";
import { AttemptMonitor } from "../components/AttemptMonitor";
import { IntentMonitor } from "../components/IntentMonitor";
import { ReconciliationMonitor } from "../components/ReconciliationMonitor";
import { WebhookEventMonitor } from "../components/WebhookEventMonitor";
import type {
  CreateReconciliationRunDraft,
  PaymentAttemptListFilter,
  PaymentIntentListFilter,
  PaymentIntentView,
  PaymentMonitorAdminController,
  PaymentMonitorAdminState,
  PaymentWebhookEventListFilter,
  ReconciliationRunListFilter,
} from "../types/monitor-admin-types";

export interface PaymentMonitorAdminWorkspaceProps {
  controller: PaymentMonitorAdminController;
  capabilities: PaymentMonitorAdminCapabilities;
  title?: string;
  description?: string;
}

export interface PaymentMonitorAdminCapabilities {
  canCreateReconciliationRun: boolean;
  canReplayWebhookEvent: boolean;
}

type TabKind = "intents" | "attempts" | "webhooks" | "reconciliation";

export function PaymentMonitorAdminWorkspace(
  props: PaymentMonitorAdminWorkspaceProps,
) {
  const { controller } = props;
  const [state, setState] = React.useState<PaymentMonitorAdminState>(() =>
    controller.getState(),
  );
  const [tab, setTab] = React.useState<TabKind>("intents");

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

  return (
    <PaymentAdminI18nBoundary>
      <PaymentAdminWorkspace
        data-slot="payment-monitor-admin-workspace"
        description={props.description}
        error={state.lastError}
        title={props.title ?? "Payment operations monitor"}
      >
        <Tabs value={tab} onValueChange={(value) => setTab(value as TabKind)}>
          <PaymentAdminTabsList aria-label="Payment operation sections">
            <PaymentAdminTabsTrigger value="intents">Intents</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="attempts">Attempts</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="webhooks">Webhook events</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="reconciliation">Reconciliation</PaymentAdminTabsTrigger>
          </PaymentAdminTabsList>

          <PaymentAdminTabsContent value="intents">
            <IntentMonitor
              intents={state.intents}
              pageInfo={state.listPageInfo?.intents}
              busy={busy}
              selectedIntent={state.selectedIntentDetail}
              onApplyFilter={handleApplyIntentFilter}
              onLoadMore={() => void controller.loadMoreIntents()}
              onSelect={handleSelectIntent}
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
      </PaymentAdminWorkspace>
    </PaymentAdminI18nBoundary>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentMonitorAdminTabs, TabsList, TabsTrigger, TabsContent };
