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
  capabilities: PaymentChannelAdminCapabilities;
  section?: PaymentChannelAdminSection;
  title?: string;
  description?: string;
}

export interface PaymentChannelAdminCapabilities {
  canCreateMethod: boolean;
  canUpdateMethod: boolean;
  canCreateChannel: boolean;
  canCreateRouteRule: boolean;
  canUpdateRouteRule: boolean;
  canDeleteRouteRule: boolean;
}

export type PaymentChannelAdminSection = "methods" | "channels" | "rules";

export function PaymentChannelAdminWorkspace(
  props: PaymentChannelAdminWorkspaceProps,
) {
  const { controller } = props;
  const [state, setState] = React.useState<PaymentChannelAdminState>(() =>
    controller.getState(),
  );
  const [tab, setTab] = React.useState<PaymentChannelAdminSection>("methods");
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
    if (!props.section) {
      setTab("channels");
    }
  }

  return (
    <PaymentAdminI18nBoundary>
      <PaymentAdminWorkspace
        data-slot="payment-channel-admin-workspace"
        description={props.description}
        error={state.lastError}
        title={props.title ?? "Payment methods, channels & routing"}
      >
        <Tabs
          value={activeSection}
          onValueChange={(value) => {
            if (!props.section) {
              setTab(value as PaymentChannelAdminSection);
            }
          }}
        >
          {!props.section ? (
            <PaymentAdminTabsList aria-label="Payment channel sections">
              <PaymentAdminTabsTrigger value="methods">Payment methods</PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger value="channels">Channels</PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger value="rules">Route rules</PaymentAdminTabsTrigger>
            </PaymentAdminTabsList>
          ) : null}

          <PaymentAdminTabsContent value="methods">
            <PaymentMethodManager
              methods={state.methods}
              pageInfo={state.listPageInfo?.methods}
              busy={busy}
              selectedId={state.selectedMethodId}
              canCreate={props.capabilities.canCreateMethod}
              canUpdate={props.capabilities.canUpdateMethod}
              onSelect={(method) => handleSelectMethod(method.id)}
              onCreate={handleCreateMethod}
              onUpdate={handleUpdateMethod}
              onLoadMore={() => void controller.loadMoreMethods()}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="channels">
            <ChannelManager
              channels={state.channels}
              methods={state.methods}
              providerAccounts={state.providerAccounts}
              pageInfo={state.listPageInfo?.channels}
              busy={busy}
              canCreate={props.capabilities.canCreateChannel}
              onCreate={handleCreateChannel}
              onLoadMore={() => void controller.loadMoreChannels()}
            />
          </PaymentAdminTabsContent>

          <PaymentAdminTabsContent value="rules">
            <RouteRuleManager
              routeRules={state.routeRules}
              channels={state.channels}
              pageInfo={state.listPageInfo?.routeRules}
              busy={busy}
              canCreate={props.capabilities.canCreateRouteRule}
              canDelete={props.capabilities.canDeleteRouteRule}
              canUpdate={props.capabilities.canUpdateRouteRule}
              onCreate={handleCreateRouteRule}
              onUpdate={handleUpdateRouteRule}
              onDelete={handleDeleteRouteRule}
              onLoadMore={() => void controller.loadMoreRouteRules()}
            />
          </PaymentAdminTabsContent>
        </Tabs>
      </PaymentAdminWorkspace>
    </PaymentAdminI18nBoundary>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentChannelAdminTabs, TabsList, TabsTrigger, TabsContent };
