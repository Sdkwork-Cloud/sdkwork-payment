import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { SdkworkPaymentListPaginationControls } from "../src";

afterEach(cleanup);

describe("payment list pagination controls", () => {
  it("keeps the result summary visible on the final page", () => {
    render(
      <SdkworkPaymentListPaginationControls
        pageInfo={{
          hasMore: false,
          mode: "offset",
          page: 2,
          pageSize: 20,
          totalItems: "42",
        }}
      />,
    );

    expect(screen.getByText("Showing 40 of 42")).toBeVisible();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("loads the next page through an accessible action", () => {
    const onLoadMore = vi.fn();
    render(
      <SdkworkPaymentListPaginationControls
        label="Load more payments"
        onLoadMore={onLoadMore}
        pageInfo={{ hasMore: true, mode: "cursor", nextCursor: "next-page" }}
        summary="Showing 20 of 42"
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Load more payments" }));

    expect(onLoadMore).toHaveBeenCalledOnce();
  });

  it("does not render an empty footer without pagination information", () => {
    const { container } = render(<SdkworkPaymentListPaginationControls />);

    expect(container).toBeEmptyDOMElement();
  });
});
