/**
 * Shared confirmation dialog for admin destructive/dangerous actions.
 *
 * Replaces `window.confirm` across all admin capability packages with a
 * styled, accessible, keyboard-navigable Dialog. Mirrors industry PSP
 * confirmation patterns (Stripe Dashboard delete confirmation, Adyen
 * Customer Area revoke confirmation).
 *
 * Usage:
 *   const [confirm, setConfirm] = useState<ConfirmDialogState>();
 *   <ConfirmDialog
 *     open={confirm !== undefined}
 *     title={confirm?.title}
 *     description={confirm?.description}
 *     confirmLabel={confirm?.confirmLabel}
 *     variant={confirm?.variant}
 *     onConfirm={() => { ...confirm?.onConfirm(); setConfirm(undefined); }}
 *     onOpenChange={(open) => { if (!open) setConfirm(undefined); }}
 *   />
 */

import * as React from "react";
import { Button, Dialog, DialogContent, DialogHeader, DialogTitle } from "@sdkwork/ui-pc-react";

export type ConfirmDialogVariant = "danger" | "warning" | "primary";

export interface ConfirmDialogProps {
  open: boolean;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: ConfirmDialogVariant;
  busy?: boolean;
  onConfirm(): void | Promise<void>;
  onOpenChange(open: boolean): void;
}

const VARIANT_BUTTON: Record<ConfirmDialogVariant, "primary" | "danger"> = {
  danger: "danger",
  warning: "primary",
  primary: "primary",
};

export function ConfirmDialog(props: ConfirmDialogProps) {
  const [busy, setBusy] = React.useState(false);
  const variant = props.variant ?? "danger";
  const confirmLabel = props.confirmLabel ?? (variant === "danger" ? "Delete" : "Confirm");
  const cancelLabel = props.cancelLabel ?? "Cancel";

  async function handleConfirm() {
    if (busy) return;
    setBusy(true);
    try {
      await props.onConfirm();
    } finally {
      setBusy(false);
    }
  }

  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{props.title}</DialogTitle>
        </DialogHeader>
        {props.description ? (
          <p className="text-sm text-[var(--sdk-color-text-secondary)]">{props.description}</p>
        ) : null}
        <div className="flex justify-end gap-2 pt-2">
          <Button
            type="button"
            variant="ghost"
            onClick={() => props.onOpenChange(false)}
            disabled={busy || props.busy}
            title={cancelLabel}
          >
            {cancelLabel}
          </Button>
          <Button
            type="button"
            variant={VARIANT_BUTTON[variant]}
            onClick={() => void handleConfirm()}
            disabled={busy || props.busy}
            title={confirmLabel}
          >
            {busy ? "Processing..." : confirmLabel}
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}

/**
 * Convenience state holder for components that need a single confirm dialog.
 * Usage:
 *   const confirm = useConfirmDialog();
 *   confirm.open({ title: "Delete?", description: "...", onConfirm: async () => { ... } });
 *   <ConfirmDialog {...confirm.dialogProps} />
 */
export interface ConfirmDialogState {
  open: boolean;
  title: string;
  description?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  variant?: ConfirmDialogVariant;
  onConfirm: () => void | Promise<void>;
}

export interface UseConfirmDialogResult {
  dialogProps: ConfirmDialogProps;
  open(state: Omit<ConfirmDialogState, "open">): void;
  close(): void;
}

export function useConfirmDialog(): UseConfirmDialogResult {
  const [state, setState] = React.useState<ConfirmDialogState | null>(null);

  return {
    dialogProps: {
      open: state !== null,
      title: state?.title ?? "",
      description: state?.description,
      confirmLabel: state?.confirmLabel,
      cancelLabel: state?.cancelLabel,
      variant: state?.variant,
      onConfirm: async () => {
        if (state?.onConfirm) {
          await state.onConfirm();
        }
        setState(null);
      },
      onOpenChange: (open) => {
        if (!open) setState(null);
      },
    },
    open: (next) =>
      setState({
        open: true,
        ...next,
      }),
    close: () => setState(null),
  };
}
