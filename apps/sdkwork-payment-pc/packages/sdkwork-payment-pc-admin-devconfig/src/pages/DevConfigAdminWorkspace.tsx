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
  SettingsSection,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@sdkwork/ui-pc-react";
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
    <section className="space-y-6" data-slot="payment-devconfig-admin-workspace">
      <header className="space-y-2">
        <h2 className="text-lg font-semibold text-[var(--sdk-color-text)]">
          {props.title ?? "Payment development configuration"}
        </h2>
        <p className="text-sm text-[var(--sdk-color-text-secondary)]">
          {props.description ??
            "Centralized developer surface for provider credentials, webhook debugging, certificate management, and integration logs. Aligned with Stripe CLI, Alipay open platform, and WeChat Pay dev tools."}
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

      <Tabs
        value={tab}
        onValueChange={(value) => setTab(value as TabKind)}
      >
        <TabsList>
          <TabsTrigger value="environment">Environment &amp; Test</TabsTrigger>
          <TabsTrigger value="webhook">Webhook Debugger</TabsTrigger>
          <TabsTrigger value="certificates">Certificates</TabsTrigger>
          <TabsTrigger value="logs">Integration Logs</TabsTrigger>
        </TabsList>

        <TabsContent value="environment">
          <SettingsSection
            title="Environment switching & credential test"
            description="Switch provider account environments (development / sandbox / production) and verify connectivity via the lowest-cost PSP API. Production transitions require elevated backend permission."
          >
            <EnvironmentSwitcher
              accounts={state.providerAccounts}
              pageInfo={state.listPageInfo?.providerAccounts}
              busy={busy}
              lastTestResult={state.lastTestResult}
              onSwitchEnvironment={handleSwitchEnvironment}
              onTest={handleTestProviderAccount}
              onLoadMore={() => void controller.loadMoreProviderAccounts()}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="webhook">
          <SettingsSection
            title="Webhook debugger"
            description="Simulate PSP webhook events for local/sandbox integration and verify webhook signatures against the configured webhook_secret_ref. Mirrors Stripe CLI trigger + listen."
          >
            <WebhookDebugger
              accounts={state.providerAccounts}
              recentEvents={state.webhookEvents}
              busy={busy}
              lastSandboxTriggerResult={state.lastSandboxTriggerResult}
              lastSignatureTestResult={state.lastSignatureTestResult}
              onSandboxTrigger={handleSandboxTrigger}
              onSignatureTest={handleSignatureTest}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="certificates">
          <SettingsSection
            title="Certificate management"
            description="PEM certificate reference registry. PEM content is never stored; only the env var reference and parsed metadata (subject CN, serial, fingerprint, validity) are persisted."
          >
            <CertificateManager
              certificates={state.certificates}
              pageInfo={state.listPageInfo?.certificates}
              busy={busy}
              onCreate={handleCreateCertificate}
              onDelete={handleDeleteCertificate}
              onLoadMore={() => void controller.loadMoreCertificates()}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="logs">
          <SettingsSection
            title="Integration logs"
            description="Full webhook event timeline with replay capability. Filters are pushed to the server (PAGINATION_SPEC.md §2). Replay is capped at 5 retries per event."
          >
            <IntegrationLogs
              events={state.webhookEvents}
              pageInfo={state.listPageInfo?.webhookEvents}
              busy={busy}
              lastReplayResult={state.lastReplayResult}
              onApplyFilter={handleApplyWebhookFilter}
              onLoadMore={() => void controller.loadMoreWebhookEvents()}
              onReplay={handleReplayWebhook}
            />
          </SettingsSection>
        </TabsContent>
      </Tabs>
    </section>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentDevConfigAdminTabs, TabsList, TabsTrigger, TabsContent };


