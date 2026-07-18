/**
 * Dev config admin workspace.
 *
 * Four-tab workspace aligned with industry PSP admin consoles (Stripe
 * Dashboard → Developers, Alipay open platform → Dev config, WeChat Pay
 * merchant platform → Dev center):
 *
 *   1. Environment Switcher — switch provider account environments
 *      (development/sandbox/production) + credential test
 *   2. Webhook Debugger — sandbox trigger + signature verification
 *   3. Certificate Manager — PEM certificate reference CRUD + expiry warnings
 *   4. Integration Logs — full webhook event timeline + replay
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
import { CertificateManager } from "../components/CertificateManager";
import { EnvironmentSwitcher } from "../components/EnvironmentSwitcher";
import { IntegrationLogs } from "../components/IntegrationLogs";
import { WebhookDebugger } from "../components/WebhookDebugger";
import type {
  PaymentCertificateDraft,
  PaymentDevConfigAdminController,
  PaymentDevConfigAdminState,
  PaymentDevSandboxTriggerDraft,
  PaymentDevWebhookSignatureTestDraft,
  PaymentProviderEnvironment,
  PaymentWebhookEventListFilter,
} from "../types/devconfig-admin-types";

export interface PaymentDevConfigAdminWorkspaceProps {
  controller: PaymentDevConfigAdminController;
  title?: string;
  description?: string;
}

type TabKind = "environment" | "webhook" | "certificates" | "logs";

export function PaymentDevConfigAdminWorkspace(
  props: PaymentDevConfigAdminWorkspaceProps,
) {
  const { controller } = props;
  const [state, setState] = React.useState<PaymentDevConfigAdminState>(() =>
    controller.getState(),
  );
  const [tab, setTab] = React.useState<TabKind>("environment");

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

  const busy =
    state.status === "loading" ||
    state.status === "saving" ||
    state.status === "testing";

  async function handleSwitchEnvironment(
    id: string,
    environment: PaymentProviderEnvironment,
  ) {
    await controller.switchProviderAccountEnvironment(id, environment);
  }

  async function handleTestProviderAccount(id: string) {
    await controller.testProviderAccount(id, { dryRun: false });
  }

  async function handleSandboxTrigger(
    providerAccountId: string,
    eventType: string,
    overrides: { amount?: string; currencyCode?: string; outTradeNo?: string },
  ) {
    const draft: PaymentDevSandboxTriggerDraft = {
      providerAccountId,
      eventType,
      ...overrides,
    };
    await controller.triggerSandboxEvent(draft);
  }

  async function handleSignatureTest(
    providerAccountId: string,
    payload: string,
    signature: string,
    timestamp: string,
    signatureHeader: string,
  ) {
    const draft: PaymentDevWebhookSignatureTestDraft = {
      providerAccountId,
      payload,
      signature,
      ...(timestamp ? { timestamp } : {}),
      ...(signatureHeader ? { signatureHeader } : {}),
    };
    await controller.testWebhookSignature(draft);
  }

  async function handleCreateCertificate(draft: PaymentCertificateDraft) {
    await controller.createCertificate(draft);
  }

  async function handleDeleteCertificate(id: string) {
    // Confirmation is handled inside CertificateManager via ConfirmDialog.
    await controller.deleteCertificate(id);
  }

  async function handleApplyWebhookFilter(filter: PaymentWebhookEventListFilter) {
    await controller.loadMoreWebhookEvents(filter);
  }

  async function handleReplayWebhook(eventId: string) {
    await controller.replayWebhookEvent(eventId);
  }

  return (
    <PaymentAdminI18nBoundary>
      <PaymentAdminWorkspace
        data-slot="payment-devconfig-admin-workspace"
        description={props.description}
        error={state.lastError}
        title={props.title ?? "Payment integration configuration"}
      >
        <Tabs value={tab} onValueChange={(value) => setTab(value as TabKind)}>
          <PaymentAdminTabsList aria-label="Payment developer tool sections">
            <PaymentAdminTabsTrigger value="environment">Environment &amp; Test</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="webhook">Webhook Debugger</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="certificates">Certificates</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="logs">Integration Logs</PaymentAdminTabsTrigger>
          </PaymentAdminTabsList>

          <PaymentAdminTabsContent value="environment">
            <EnvironmentSwitcher
              accounts={state.providerAccounts}
              pageInfo={state.listPageInfo?.providerAccounts}
              busy={busy}
              lastTestResult={state.lastTestResult}
              onSwitchEnvironment={handleSwitchEnvironment}
              onTest={handleTestProviderAccount}
              onLoadMore={() => void controller.loadMoreProviderAccounts()}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="webhook">
            <WebhookDebugger
              accounts={state.providerAccounts}
              recentEvents={state.webhookEvents}
              busy={busy}
              lastSandboxTriggerResult={state.lastSandboxTriggerResult}
              lastSignatureTestResult={state.lastSignatureTestResult}
              onSandboxTrigger={handleSandboxTrigger}
              onSignatureTest={handleSignatureTest}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="certificates">
            <CertificateManager
              certificates={state.certificates}
              pageInfo={state.listPageInfo?.certificates}
              busy={busy}
              onCreate={handleCreateCertificate}
              onDelete={handleDeleteCertificate}
              onLoadMore={() => void controller.loadMoreCertificates()}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="logs">
            <IntegrationLogs
              events={state.webhookEvents}
              pageInfo={state.listPageInfo?.webhookEvents}
              busy={busy}
              lastReplayResult={state.lastReplayResult}
              onApplyFilter={handleApplyWebhookFilter}
              onLoadMore={() => void controller.loadMoreWebhookEvents()}
              onReplay={handleReplayWebhook}
            />
          </PaymentAdminTabsContent>
        </Tabs>
      </PaymentAdminWorkspace>
    </PaymentAdminI18nBoundary>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentDevConfigAdminTabs, TabsList, TabsTrigger, TabsContent };
