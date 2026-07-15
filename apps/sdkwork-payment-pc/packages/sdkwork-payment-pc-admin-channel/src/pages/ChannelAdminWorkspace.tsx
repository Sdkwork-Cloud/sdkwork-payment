/**
 * Channel admin workspace.
 *
 * Three-tab workspace for payment method / channel / routing rule management:
 *   1. Payment Methods — list + create + edit (no delete per API)
 *   2. Channels — list + create only (no update/delete per API)
 *   3. Routing Rules — list + create + update + delete
 *
 * Mirrors industry PSP admin consoles (Stripe Dashboard → Payment methods +
 * Routing, Adyen → Payment method config, WeChat Pay → product center).
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
import { ChannelManager } from "../components/ChannelManager";
import { PaymentMethodManager } from "../components/PaymentMethodManager";
import { RouteRuleManager } from "../components/RouteRuleManager";
import type {
  PaymentChannelAdminController,
  PaymentChannelAdminState,
  PaymentChannelDraft,
  PaymentMethodDraft,
  PaymentMethodUpdateDraft,
  PaymentRouteRuleDraft,
  PaymentRouteRuleUpdateDraft,
} from "../types/channel-admin-types";

export interface PaymentChannelAdminWorkspaceProps {
  controller: PaymentChannelAdminController;
  title?: string;
  description?: string;
}

type TabKind = "methods" | "channels" | "rules";

export function PaymentChannelAdminWorkspace(
  props: PaymentChannelAdminWorkspaceProps,
) {
  const { controller } = props;
  const [state, setState] = React.useState<PaymentChannelAdminState>(() =>
    controller.getState(),
  );
  const [tab, setTab] = React.useState<TabKind>("methods");

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

  async function handleCreateMethod(draft: PaymentMethodDraft) {
    await controller.createMethod(draft);
  }

  async function handleUpdateMethod(methodKey: string, draft: PaymentMethodUpdateDraft) {
    await controller.updateMethod(methodKey, draft);
  }

  async function handleCreateChannel(draft: PaymentChannelDraft) {
    await controller.createChannel(draft);
  }

  async function handleCreateRouteRule(draft: PaymentRouteRuleDraft) {
    await controller.createRouteRule(draft);
  }

  async function handleUpdateRouteRule(id: string, draft: PaymentRouteRuleUpdateDraft) {
    await controller.updateRouteRule(id, draft);
  }

  async function handleDeleteRouteRule(id: string) {
    await controller.deleteRouteRule(id);
  }

  function handleSelectMethod(methodId: string) {
    controller.selectMethod(methodId);
    setTab("channels");
  }

  return (
    <section className="space-y-6" data-slot="payment-channel-admin-workspace">
      <header className="space-y-2">
        <h2 className="text-lg font-semibold text-[var(--sdk-color-text)]">
          {props.title ?? "Payment channels & routing"}
        </h2>
        <p className="text-sm text-[var(--sdk-color-text-secondary)]">
          {props.description ??
            "Configure payment methods, channels, and routing rules. A channel bridges a method with a provider account; routing rules match payment requests to channels."}
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
          <TabsTrigger value="methods">Payment Methods</TabsTrigger>
          <TabsTrigger value="channels">Channels</TabsTrigger>
          <TabsTrigger value="rules">Routing Rules</TabsTrigger>
        </TabsList>

        <TabsContent value="methods">
          <SettingsSection
            title="Payment methods"
            description="The 'what' of payments — user-facing payment instruments (alipay_wap, wechat_h5, stripe_card, etc.). Each method is bridged to a provider account via a channel."
            actions={null}
          >
            <PaymentMethodManager
              methods={state.methods}
              pageInfo={state.listPageInfo?.methods}
              busy={busy}
              selectedId={state.selectedMethodId}
              onSelect={(method) => handleSelectMethod(method.id)}
              onCreate={handleCreateMethod}
              onUpdate={handleUpdateMethod}
              onLoadMore={() => void controller.loadMoreMethods()}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="channels">
          <SettingsSection
            title="Payment channels"
            description="The 'how' of payments — bridges a payment method with a provider account under a specific scene (app / web / mini_program / api). Channels cannot be edited or deleted via the API — set status carefully at creation."
            actions={null}
          >
            <ChannelManager
              channels={state.channels}
              methods={state.methods}
              providerAccounts={state.providerAccounts}
              pageInfo={state.listPageInfo?.channels}
              busy={busy}
              onCreate={handleCreateChannel}
              onLoadMore={() => void controller.loadMoreChannels()}
            />
          </SettingsSection>
        </TabsContent>

        <TabsContent value="rules">
          <SettingsSection
            title="Routing rules"
            description="The 'traffic controller' — match payment requests to channels based on conditions (purchase type, country, currency, amount range, user segment, risk level). Lower priority numbers win."
            actions={null}
          >
            <RouteRuleManager
              routeRules={state.routeRules}
              channels={state.channels}
              pageInfo={state.listPageInfo?.routeRules}
              busy={busy}
              onCreate={handleCreateRouteRule}
              onUpdate={handleUpdateRouteRule}
              onDelete={handleDeleteRouteRule}
              onLoadMore={() => void controller.loadMoreRouteRules()}
            />
          </SettingsSection>
        </TabsContent>
      </Tabs>
    </section>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentChannelAdminTabs, TabsList, TabsTrigger, TabsContent };
