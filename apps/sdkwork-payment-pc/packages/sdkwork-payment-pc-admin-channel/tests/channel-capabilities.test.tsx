import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ChannelManager } from "../src/components/ChannelManager";
import { PaymentMethodManager } from "../src/components/PaymentMethodManager";
import { RouteRuleManager } from "../src/components/RouteRuleManager";

afterEach(cleanup);

describe("payment channel capabilities", () => {
  it("hides method and channel creation for read-only operators", () => {
    const { unmount } = render(
      <PaymentMethodManager
        canCreate={false}
        canUpdate={false}
        methods={[{
          id: "method-1",
          methodKey: "stripe_card",
          displayName: "Card",
          providerCode: "stripe",
          currencyCode: "USD",
          scope: "global",
          status: "active",
          sortOrder: 0,
        } as never]}
        onCreate={vi.fn()}
        onLoadMore={vi.fn()}
        onSelect={vi.fn()}
        onUpdate={vi.fn()}
      />,
    );
    expect(screen.queryByRole("button", { name: "Create payment method" })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Select" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    unmount();

    render(
      <ChannelManager
        canCreate={false}
        channels={[]}
        methods={[]}
        onCreate={vi.fn()}
        onLoadMore={vi.fn()}
        providerAccounts={[]}
      />,
    );
    expect(screen.queryByRole("button", { name: "Create channel" })).not.toBeInTheDocument();
  });

  it("hides route-rule create, edit, and delete for read-only operators", () => {
    render(
      <RouteRuleManager
        canCreate={false}
        canDelete={false}
        canUpdate={false}
        channels={[{ id: "channel-1", channelNo: "channel-main" } as never]}
        onCreate={vi.fn()}
        onDelete={vi.fn()}
        onLoadMore={vi.fn()}
        onUpdate={vi.fn()}
        routeRules={[{
          id: "rule-1",
          ruleNo: "route-main",
          channelId: "channel-1",
          priority: 1,
          status: "active",
        } as never]}
      />,
    );

    expect(screen.getByText("route-main")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Create route rule" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Delete" })).not.toBeInTheDocument();
  });
});
