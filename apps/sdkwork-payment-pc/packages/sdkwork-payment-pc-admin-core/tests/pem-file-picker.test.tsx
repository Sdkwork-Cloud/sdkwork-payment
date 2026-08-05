import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { PemFilePicker } from "../src";

const PEM_CONTENT = [
  "-----BEGIN PRIVATE KEY-----",
  "MIIEvQIBADANBgkqhkiG9w0BAQEFAASC",
  "-----END PRIVATE KEY-----",
  "",
].join("\n");

function selectFile(input: HTMLInputElement, content: string, name = "merchant-private-key.pem") {
  const file = new File([content], name, { type: "text/plain" });
  fireEvent.change(input, { target: { files: [file] } });
}

afterEach(cleanup);

describe("PemFilePicker", () => {
  it("opens the file picker when the upload button is clicked", () => {
    const clickSpy = vi
      .spyOn(HTMLInputElement.prototype, "click")
      .mockImplementation(() => {});
    const onContent = vi.fn();

    render(<PemFilePicker onContent={onContent} />);
    fireEvent.click(screen.getByRole("button", { name: "Upload file" }));

    expect(clickSpy).toHaveBeenCalledTimes(1);
    clickSpy.mockRestore();
  });

  it("reads a selected file and passes its text content to onContent", async () => {
    const onContent = vi.fn();
    const { container } = render(<PemFilePicker onContent={onContent} />);
    const input = container.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();

    selectFile(input as HTMLInputElement, PEM_CONTENT);

    await waitFor(() => {
      expect(onContent).toHaveBeenCalledWith(PEM_CONTENT);
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it("rejects a file above maxBytes without calling onContent", async () => {
    const onContent = vi.fn();
    const onError = vi.fn();
    const { container } = render(
      <PemFilePicker maxBytes={64} onContent={onContent} onError={onError} />,
    );
    const input = container.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();

    selectFile(input as HTMLInputElement, PEM_CONTENT);

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("64-byte limit");
    });
    expect(onContent).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledTimes(1);
  });

  it("rejects an empty file without calling onContent", async () => {
    const onContent = vi.fn();
    const onError = vi.fn();
    const { container } = render(<PemFilePicker onContent={onContent} onError={onError} />);
    const input = container.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();

    selectFile(input as HTMLInputElement, "");

    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("empty");
    });
    expect(onContent).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledTimes(1);
  });

  it("supports selecting another file after a rejected read", async () => {
    const onContent = vi.fn();
    const { container } = render(<PemFilePicker maxBytes={64} onContent={onContent} />);
    const input = container.querySelector<HTMLInputElement>('input[type="file"]');
    expect(input).not.toBeNull();

    selectFile(input as HTMLInputElement, "x".repeat(128));
    await waitFor(() => {
      expect(screen.getByRole("alert")).toHaveTextContent("64-byte limit");
    });

    const smallPem = "-----BEGIN PUBLIC KEY-----\nabc\n-----END PUBLIC KEY-----\n";
    selectFile(input as HTMLInputElement, smallPem);
    await waitFor(() => {
      expect(onContent).toHaveBeenCalledWith(smallPem);
    });
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });
});
