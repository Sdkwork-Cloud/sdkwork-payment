import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { Tabs } from "@sdkwork/ui-pc-react";

import {
  PaymentAdminTabsContent,
  PaymentAdminTabsList,
  PaymentAdminTabsTrigger,
  PaymentAdminWorkspace,
} from "../src";

afterEach(cleanup);

describe("payment admin workspace", () => {
  it("renders a labelled workspace without a visual page header", () => {
    render(
      <PaymentAdminWorkspace error="Unable to load providers" title="Payment providers">
        <div>Workspace content</div>
      </PaymentAdminWorkspace>,
    );

    expect(screen.getByRole("region", { name: "Payment providers" })).toBeVisible();
    expect(screen.queryByRole("heading", { level: 1 })).not.toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("Unable to load providers");
    expect(screen.queryByText(/Configure Stripe/i)).not.toBeInTheDocument();
  });

  it("switches sections through accessible tabs", () => {
    render(
      <PaymentAdminWorkspace title="Payment operations">
        <Tabs defaultValue="intents">
          <PaymentAdminTabsList aria-label="Payment operation sections">
            <PaymentAdminTabsTrigger value="intents">Intents</PaymentAdminTabsTrigger>
            <PaymentAdminTabsTrigger value="webhooks">Webhooks</PaymentAdminTabsTrigger>
          </PaymentAdminTabsList>
          <PaymentAdminTabsContent value="intents">Intent list</PaymentAdminTabsContent>
          <PaymentAdminTabsContent value="webhooks">Webhook list</PaymentAdminTabsContent>
        </Tabs>
      </PaymentAdminWorkspace>,
    );

    expect(screen.getByRole("tablist", { name: "Payment operation sections" })).toBeVisible();
    expect(screen.getByText("Intent list")).toBeVisible();

    fireEvent.mouseDown(screen.getByRole("tab", { name: "Webhooks" }), {
      button: 0,
      ctrlKey: false,
    });

    expect(screen.getByRole("tab", { name: "Webhooks" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByText("Webhook list")).toBeVisible();
  });
});
