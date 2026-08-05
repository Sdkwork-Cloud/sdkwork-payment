/**
 * PEM / credential file picker for admin credential fields.
 *
 * Lets operators click a link-style button below a credential input to select
 * a local PEM, key, or certificate file and reads its UTF-8 text content into
 * the owning form field — mirroring industry PSP surfaces where operators
 * upload the key/cert file instead of pasting long PEM bodies by hand (Alipay
 * merchant private key, WeChat platform certificate, Stripe secret key files).
 *
 * The link presentation keeps the upload as a secondary action below the
 * field; the input itself remains the primary editing surface. The file is
 * read entirely in the browser; nothing is uploaded until the form itself is
 * submitted, exactly like a manual paste. Read failures, empty files, and
 * files above `maxBytes` are rejected with an inline error and never
 * overwrite the field's existing content.
 */

import * as React from "react";
import { FileUp } from "lucide-react";

export interface PemFilePickerProps {
  /** Link label. Defaults to "Upload file". */
  label?: string;
  /** File accept filter forwarded to the hidden file input. */
  accept?: string;
  /**
   * Maximum accepted file size in bytes. Larger files are rejected with an
   * inline error and do not overwrite the field. Backend credential limits
   * are 32768 bytes (secrets) / 65536 bytes (certificates).
   */
  maxBytes?: number;
  disabled?: boolean;
  /** Called with the file's UTF-8 text content after a successful read. */
  onContent(content: string): void;
  /** Called when the selected file is rejected (empty / too large / unreadable). */
  onError?(message: string): void;
}

const DEFAULT_ACCEPT = ".pem,.crt,.cer,.key,.pub,.txt,application/x-pem-file,text/plain";

export function PemFilePicker(props: PemFilePickerProps) {
  const inputRef = React.useRef<HTMLInputElement | null>(null);
  const [reading, setReading] = React.useState(false);
  const [error, setError] = React.useState<string | undefined>();

  function openFilePicker() {
    inputRef.current?.click();
  }

  function reject(message: string) {
    setError(message);
    props.onError?.(message);
  }

  function handleFileChange(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    // Reset the input so selecting the same file again re-triggers change.
    event.target.value = "";
    if (!file) return;

    if (props.maxBytes !== undefined && file.size > props.maxBytes) {
      reject(`File exceeds the ${props.maxBytes}-byte limit.`);
      return;
    }
    if (file.size === 0) {
      reject("Selected file is empty.");
      return;
    }

    setReading(true);
    setError(undefined);
    const reader = new FileReader();
    reader.onload = () => {
      setReading(false);
      const content = typeof reader.result === "string" ? reader.result : "";
      if (!content.trim()) {
        reject("Selected file contains no text content.");
        return;
      }
      props.onContent(content);
    };
    reader.onerror = () => {
      setReading(false);
      reject("Failed to read the selected file.");
    };
    reader.readAsText(file, "utf-8");
  }

  const label = props.label ?? "Upload file";

  return (
    <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
      <button
        type="button"
        onClick={openFilePicker}
        disabled={props.disabled || reading}
        title={reading ? "Reading file..." : label}
        className="inline-flex items-center gap-1 text-xs font-medium text-[var(--sdk-color-brand-primary)] underline underline-offset-4 hover:text-[var(--sdk-color-brand-primary-hover)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--sdk-color-border-focus)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--sdk-color-surface-canvas)] disabled:pointer-events-none disabled:opacity-60"
      >
        <FileUp className="h-3 w-3" aria-hidden="true" />
        {reading ? "Reading..." : label}
      </button>
      <input
        ref={inputRef}
        type="file"
        accept={props.accept ?? DEFAULT_ACCEPT}
        className="hidden"
        onChange={handleFileChange}
        tabIndex={-1}
        aria-hidden="true"
      />
      {error ? (
        <span role="alert" className="text-xs text-[var(--sdk-color-text-error)]">
          {error}
        </span>
      ) : null}
    </div>
  );
}
