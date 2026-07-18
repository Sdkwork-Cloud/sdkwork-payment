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
  SettingsSection,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@sdkwork/ui-pc-react";
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
    <section className="space-y-6" data-slot="payment-monitor-admin-workspace">
      <header className="space-y-2">
        <h2 className="text-lg font-semibold text-[var(--sdk-color-text)]">
          {props.title ?? "Payment operations monitor"}
        </h2>
        <p className="text-sm text-[var(--sdk-color-text-secondary)]">
          {props.description ??
            "Monitor payment intents, attempts, webhook events, and reconciliation runs. Investigate failures, replay stuck webhooks, and trigger reconciliation cycles."}
        </p>
      </header>

      {state.lastError ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {state.lastError}
        </div>
      ) : null}

      <Tabs value={tab} onValueChange={(value) => setTab(value as TabKind)}>
        <TabsList>
          <TabsTrigger value="intents">Intents</TabsTrigger>
          <TabsTrigger value="attempts">Attempts</TabsTrigger>
          <TabsTrigger value="webhooks">Webhook Events</TabsTrigger>
          <TabsTrigger value="reconciliation">Reconciliation</TabsTrigger>
        </TabsList>

        <TabsContent value="intents">
          <SettingsSection
            title="Payment intents"
            description="The 'why' of a payment — the user-facing payment intention tied to an order. Click an intent to view its attempts and metadata."
            actions={null}
          >
            <IntentMonitor
              intents={state.intents}
              pageInfo={state.listPageInfo?.intents}
              busy={busy}
              selectedIntent={state.selectedIntentDetail}
              onApplyFilter={handleApplyIntentFilter}
              onLoadMore={() => void controller.loadMoreIntents()}
              onSelect={handleSelectIntent}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="attempts">
          <SettingsSection
            title="Payment attempts"
            description="The 'how' of a payment — PSP-facing execution records. Each intent may produce multiple attempts (retries, provider routing)."
            actions={null}
          >
            <AttemptMonitor
              attempts={state.attempts}
              pageInfo={state.listPageInfo?.attempts}
              busy={busy}
              onApplyFilter={handleApplyAttemptFilter}
              onLoadMore={() => void controller.loadMoreAttempts()}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="webhooks">
          <SettingsSection
            title="Webhook events"
            description="Inbound event stream from PSPs. Replay stuck or failed events (up to 5 retries). Events marked 'dead' have exhausted retry limits."
            actions={null}
          >
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
          </SettingsSection>
        </TabsContent>

        <TabsContent value="reconciliation">
          <SettingsSection
            title="Reconciliation runs"
            description="Settlement matching cycles — compare local records with PSP settlement reports. Create manual runs or track scheduled ones."
            actions={null}
          >
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
          </SettingsSection>
        </TabsContent>
      </Tabs>
    </section>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentMonitorAdminTabs, TabsList, TabsTrigger, TabsContent };
