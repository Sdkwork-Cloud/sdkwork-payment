import { cleanup, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { ProviderAccountList } from "../src/components/ProviderAccountList";
import { SubMerchantManager } from "../src/components/SubMerchantManager";

const providerAccount = {
  id: "provider-1",
  accountNo: "stripe-main",
  providerCode: "stripe",
  accountMode: "partner",
  environment: "production",
  status: "active",
  capabilities: {},
  createdAt: "2026-07-17T00:00:00.000Z",
  updatedAt: "2026-07-17T00:00:00.000Z",
} as const;

afterEach(cleanup);

describe("payment provider capabilities", () => {
  it("hides provider mutation controls for read-only operators", () => {
    render(
      <ProviderAccountList
        accounts={[providerAccount as never]}
        canCreate={false}
        canEdit={false}
        canRotate={false}
        canTest={false}
        onCreate={vi.fn()}
        onEdit={vi.fn()}
        onLoadMore={vi.fn()}
        onRotate={vi.fn()}
        onSelect={vi.fn()}
        onTest={vi.fn()}
      />,
    );

    expect(screen.getByText("stripe-main")).toBeInTheDocument();
    expect(document.querySelector('[data-provider="stripe"]')).not.toBeNull();
    expect(screen.getByLabelText("Credential readiness")).toBeInTheDocument();
    for (const action of ["Create provider account", "Edit", "Rotate", "Test"]) {
      expect(screen.queryByRole("button", { name: action })).not.toBeInTheDocument();
    }
  });

  it("hides sub-merchant create, edit, and delete controls for read-only operators", () => {
    render(
      <SubMerchantManager
        canCreate={false}
        canDelete={false}
        canUpdate={false}
        onCreate={vi.fn()}
        onDelete={vi.fn()}
        onLoadMore={vi.fn()}
        onUpdate={vi.fn()}
        partnerAccount={providerAccount as never}
        subMerchants={[{
          id: "merchant-1",
          providerAccountId: "provider-1",
          subMerchantNo: "merchant-main",
          status: "active",
          createdAt: "2026-07-17T00:00:00.000Z",
          updatedAt: "2026-07-17T00:00:00.000Z",
        } as never]}
      />,
    );

    expect(screen.getAllByText("merchant-main")).toHaveLength(2);
    expect(screen.queryByRole("button", { name: /sub-merchant/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Edit" })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Delete" })).not.toBeInTheDocument();
  });
});
