/**
 * Shared copy-to-clipboard button + masked secret ref field for admin.
 *
 * Mirrors industry PSP credential display patterns (Stripe Dashboard API key
 * reveal/copy, Adyen Client Key copy, Alipay merchant private key masked
 * display). Replaces the previous pattern of showing raw env var names with
 * no copy affordance.
 */

import * as React from "react";
import { Button } from "@sdkwork/ui-pc-react";

export interface CopyButtonProps {
  value: string | undefined | null;
  label?: string;
  disabled?: boolean;
  size?: "default" | "sm" | "lg" | "icon";
  variant?: "primary" | "ghost" | "outline";
  title?: string;
}

/**
 * Copy-to-clipboard button with feedback state.
 *
 * Shows "Copy" by default, "Copied!" for 2 seconds after successful copy.
 * Uses `navigator.clipboard.writeText` with a fallback to a hidden textarea
 * + `document.execCommand("copy")` for older browsers / non-secure contexts.
 */
export function CopyButton(props: CopyButtonProps) {
  const [copied, setCopied] = React.useState(false);
  const value = props.value ?? "";
  const label = props.label ?? "Copy";
  const size = props.size ?? "sm";
  const variant = props.variant ?? "ghost";

  React.useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 2000);
    return () => clearTimeout(timer);
  }, [copied]);

  async function handleCopy() {
    if (!value || props.disabled) return;
    try {
      if (navigator.clipboard && window.isSecureContext) {
        await navigator.clipboard.writeText(value);
      } else {
        // Fallback for non-secure contexts (HTTP / older browsers)
        const textarea = document.createElement("textarea");
        textarea.value = value;
        textarea.style.position = "fixed";
        textarea.style.opacity = "0";
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand("copy");
        document.body.removeChild(textarea);
      }
      setCopied(true);
    } catch {
      // Silently fail — clipboard may be unavailable
    }
  }

  return (
    <Button
      type="button"
      variant={variant}
      size={size}
      onClick={() => void handleCopy()}
      disabled={props.disabled || !value}
      title={props.title ?? `Copy: ${value}`}
      aria-label={props.title ?? `Copy: ${value}`}
    >
      {copied ? "Copied!" : label}
    </Button>
  );
}

/**
 * Mask a secret reference, showing only the last N characters.
 *
 * Example: `ALIPAY_MERCHANT_PRIVATE_KEY` → `•••••••••••••••••••••••••••KEY`
 *
 * For short values (≤ visibleChars * 2), shows `••••` + the full value.
 */
export function maskSecretRef(
  value: string | undefined | null,
  visibleChars = 4,
): string {
  if (!value) return "—";
  if (value.length <= visibleChars * 2) {
    return `••••${value}`;
  }
  return `••••••••••••••••${value.slice(-visibleChars)}`;
}

export interface SecretRefFieldProps {
  label: string;
  value: string | undefined | null;
  /** When true, masks the value showing only last 4 chars. */
  masked?: boolean;
  /** Helper text shown below the value. */
  helperText?: string;
}

/**
 * Read-only secret reference field with reveal toggle + copy button.
 *
 * Mirrors Stripe Dashboard's API key display: shows `••••••••WXYZ` by
 * default, with a "Reveal" button to show the full env var name and a
 * "Copy" button to copy it.
 */
export function SecretRefField(props: SecretRefFieldProps) {
  const [revealed, setRevealed] = React.useState(false);
  const value = props.value ?? "";
  const displayValue = props.masked && !revealed ? maskSecretRef(value) : value;

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-medium text-[var(--sdk-color-text-muted)]">
          {props.label}
        </span>
        <div className="flex items-center gap-1">
          {props.masked ? (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setRevealed((prev) => !prev)}
              disabled={!value}
              title={revealed ? "Hide" : "Reveal"}
            >
              {revealed ? "Hide" : "Reveal"}
            </Button>
          ) : null}
          <CopyButton value={value} label="Copy" size="sm" variant="ghost" />
        </div>
      </div>
      <code className="block break-all rounded-md bg-[var(--sdk-color-bg-subtle)] px-2 py-1 font-mono text-sm text-[var(--sdk-color-text)]">
        {displayValue}
      </code>
      {props.helperText ? (
        <span className="text-xs text-[var(--sdk-color-text-secondary)]">
          {props.helperText}
        </span>
      ) : null}
    </div>
  );
}
