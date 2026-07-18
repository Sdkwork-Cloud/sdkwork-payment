/**
 * Payment method manager.
 *
 * Lists payment methods (e.g., alipay_wap, wechat_h5, stripe_card) with
 * create + edit capabilities. Payment methods are the "what" of payments —
 * the user-facing payment instrument. Channels are the "how" — which provider
 * account serves that method.
 *
 * API matrix: list + create + update (PATCH by `methodKey`). No delete.
 *
 * Mirrors industry PSP method registries (Stripe Dashboard → Payment methods,
 * Alipay open platform → product center, WeChat Pay merchant platform →
 * product center).
 */

import * as React from "react";
import {
  Badge,
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  ADMIN_PROVIDER_FORM_OPTIONS,
  ADMIN_PROVIDER_LABEL,
  adminPaymentMethodKeysForProvider,
  adminPaymentMethodKeyOption,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentEntityStatus,
  PaymentMethodDraft,
  PaymentMethodScope,
  PaymentMethodUpdateDraft,
  PaymentMethodView,
  PaymentProviderCode,
} from "../types/channel-admin-types";

export interface PaymentMethodManagerProps {
  methods: readonly PaymentMethodView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  selectedId?: string;
  canCreate: boolean;
  canUpdate: boolean;
  onSelect(method: PaymentMethodView): void;
  onCreate(draft: PaymentMethodDraft): Promise<void> | void;
  onUpdate(methodKey: string, draft: PaymentMethodUpdateDraft): Promise<void> | void;
  onLoadMore(): void;
}

const STATUS_VARIANT: Record<PaymentEntityStatus, "success" | "secondary" | "danger"> = {
  active: "success",
  inactive: "secondary",
  deprecated: "danger",
};

const SCOPE_LABEL: Record<PaymentMethodScope, string> = {
  global: "Global",
  tenant: "Tenant",
  organization: "Organization",
};

export function PaymentMethodManager(props: PaymentMethodManagerProps) {
  const [dialog, setDialog] = React.useState<
    | { kind: "closed" }
    | { kind: "create" }
    | { kind: "edit"; method: PaymentMethodView }
  >({ kind: "closed" });

  async function handleCreate(draft: PaymentMethodDraft) {
    await props.onCreate(draft);
    setDialog({ kind: "closed" });
  }

  async function handleUpdate(draft: PaymentMethodUpdateDraft) {
    if (dialog.kind !== "edit") {
      return;
    }
    await props.onUpdate(dialog.method.methodKey, draft);
    setDialog({ kind: "closed" });
  }

  return (
    <div className="space-y-4" data-slot="payment-method-manager">
      <div className="flex justify-end">
        {props.canCreate ? <Button
          type="button"
          size="sm"
          onClick={() => setDialog({ kind: "create" })}
          disabled={props.busy}
          title={props.busy ? "Cannot create a payment method while another operation is in progress" : "Create a new payment method"}
        >
          Create payment method
        </Button> : null}
      </div>

      {props.methods.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No payment methods configured. Create one to start accepting payments.
          {/* Empty-state inline create button: guides users to create a payment method directly */}
          {props.canCreate ? <div className="mt-3">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={() => setDialog({ kind: "create" })}
              disabled={props.busy}
            >
              Create payment method
            </Button>
          </div> : null}
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {/* Sort ascending by sortOrder: lower number appears first; spread into a new array to avoid mutating props */}
          {[...props.methods]
            .sort((a, b) => a.sortOrder - b.sortOrder)
            .map((method) => (
            <li
              key={method.id}
              className={
                "flex flex-col gap-2 p-4 sm:flex-row sm:items-center sm:justify-between " +
                (props.selectedId === method.id ? "bg-[var(--sdk-color-bg-subtle)]" : "")
              }
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-[var(--sdk-color-text)]">
                    {method.displayName}
                  </span>
                  <Badge variant="outline" title={method.methodKey}>
                    {adminPaymentMethodKeyOption(method.methodKey)?.label ?? method.methodKey}
                  </Badge>
                  <Badge variant="secondary">{ADMIN_PROVIDER_LABEL[method.providerCode]}</Badge>
                  <Badge variant="outline">{SCOPE_LABEL[method.scope]}</Badge>
                  <Badge variant={STATUS_VARIANT[method.status]}>{method.status}</Badge>
                </div>
                <dl className="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-3">
                  <div>
                    <dt className="inline">Currency:</dt>{" "}
                    <dd className="inline">{method.currencyCode}</dd>
                  </div>
                  <div>
                    <dt className="inline">Country:</dt>{" "}
                    <dd className="inline">{method.countryCode ?? "—"}</dd>
                  </div>
                  <div>
                    <dt className="inline">Sort order:</dt>{" "}
                    <dd className="inline">{method.sortOrder}</dd>
                  </div>
                </dl>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => props.onSelect(method)}
                  title="Cannot select while another operation is in progress"
                >
                  Select
                </Button>
                {props.canUpdate ? <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => setDialog({ kind: "edit", method })}
                  disabled={props.busy}
                  title="Cannot edit while another operation is in progress"
                >
                  Edit
                </Button> : null}
              </div>
            </li>
          ))}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
      />

      <Dialog
        open={
          (props.canCreate && dialog.kind === "create")
          || (props.canUpdate && dialog.kind === "edit")
        }
        onOpenChange={(open) => {
          if (!open) setDialog({ kind: "closed" });
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {dialog.kind === "create" ? "Create payment method" : "Edit payment method"}
            </DialogTitle>
          </DialogHeader>
          {dialog.kind === "create" || dialog.kind === "edit" ? (
            <PaymentMethodForm
              mode={dialog.kind === "create" ? "create" : "update"}
              initial={dialog.kind === "edit" ? dialog.method : undefined}
              onCancel={() => setDialog({ kind: "closed" })}
              onSubmit={
                dialog.kind === "create"
                  ? (draft) => handleCreate(draft as PaymentMethodDraft)
                  : (draft) => handleUpdate(draft as PaymentMethodUpdateDraft)
              }
            />
          ) : null}
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface PaymentMethodFormProps {
  mode: "create" | "update";
  initial?: PaymentMethodView;
  onCancel(): void;
  onSubmit(draft: PaymentMethodDraft | PaymentMethodUpdateDraft): Promise<void> | void;
}

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: PaymentEntityStatus }> = [
  { label: "Active", value: "active" },
  { label: "Inactive", value: "inactive" },
  { label: "Deprecated", value: "deprecated" },
];

const SCOPE_OPTIONS: ReadonlyArray<{ label: string; value: PaymentMethodScope }> = [
  { label: "Global", value: "global" },
  { label: "Tenant", value: "tenant" },
  { label: "Organization", value: "organization" },
];

function PaymentMethodForm(props: PaymentMethodFormProps) {
  const { mode, initial } = props;
  const [methodKey, setMethodKey] = React.useState(initial?.methodKey ?? "");
  const [displayName, setDisplayName] = React.useState(initial?.displayName ?? "");
  const [providerCode, setProviderCode] = React.useState<PaymentProviderCode>(
    initial?.providerCode ?? "alipay",
  );
  const [status, setStatus] = React.useState<PaymentEntityStatus>(initial?.status ?? "active");
  const [scope, setScope] = React.useState<PaymentMethodScope>(initial?.scope ?? "tenant");
  const [currencyCode, setCurrencyCode] = React.useState(initial?.currencyCode ?? "CNY");
  const [countryCode, setCountryCode] = React.useState(initial?.countryCode ?? "");
  const [sortOrder, setSortOrder] = React.useState(String(initial?.sortOrder ?? 0));
  const [error, setError] = React.useState<string | undefined>();

  // When provider changes in create mode, reset method_key if it doesn't
  // belong to the new provider, and auto-suggest a display name.
  function handleProviderChange(next: PaymentProviderCode) {
    setProviderCode(next);
    if (mode === "create") {
      const belongs = adminPaymentMethodKeyOption(methodKey)?.providerCode === next;
      if (!belongs) {
        setMethodKey("");
      }
    }
  }

  function handleMethodKeyChange(key: string) {
    setMethodKey(key);
    // Auto-suggest display name when empty
    if (!displayName.trim()) {
      const option = adminPaymentMethodKeyOption(key);
      if (option) {
        setDisplayName(option.label);
      }
    }
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (mode === "create" && !methodKey.trim()) {
      setError("Method key is required.");
      return;
    }
    if (!displayName.trim()) {
      setError("Display name is required.");
      return;
    }
    const sortOrderNum = Number.parseInt(sortOrder, 10);
    if (Number.isNaN(sortOrderNum)) {
      setError("Sort order must be an integer.");
      return;
    }
    try {
      if (mode === "create") {
        await props.onSubmit({
          methodKey: methodKey.trim(),
          displayName: displayName.trim(),
          providerCode,
          status,
          scope,
          currencyCode: currencyCode.trim() || "CNY",
          countryCode: countryCode.trim() || undefined,
          sortOrder: sortOrderNum,
        } as PaymentMethodDraft);
      } else {
        await props.onSubmit({
          displayName: displayName.trim(),
          providerCode,
          status,
          currencyCode: currencyCode.trim() || "CNY",
          countryCode: countryCode.trim() || undefined,
          sortOrder: sortOrderNum,
        } as PaymentMethodUpdateDraft);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save payment method.");
    }
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Provider" htmlFor="method-form-provider">
          <Select
            value={providerCode}
            onValueChange={(value) => handleProviderChange(value as PaymentProviderCode)}
          >
            <SelectTrigger id="method-form-provider">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ADMIN_PROVIDER_FORM_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        {mode === "create" ? (
          <AdminFieldLabel label="Payment method" htmlFor="method-form-key" required>
            <Select
              value={methodKey}
              onValueChange={handleMethodKeyChange}
            >
              <SelectTrigger id="method-form-key">
                <SelectValue placeholder="Select a payment method..." />
              </SelectTrigger>
              <SelectContent>
                {adminPaymentMethodKeysForProvider(providerCode).map((option) => (
                  <SelectItem key={option.methodKey} value={option.methodKey}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </AdminFieldLabel>
        ) : (
          <AdminFieldLabel label="Payment method" htmlFor="method-form-key-readonly">
            <Input
              id="method-form-key-readonly"
              value={adminPaymentMethodKeyOption(methodKey)?.label ?? methodKey}
              disabled
              readOnly
            />
          </AdminFieldLabel>
        )}
      </div>
      {mode === "create" && methodKey ? (
        <p className="text-xs text-[var(--sdk-color-text-muted)]">
          {adminPaymentMethodKeyOption(methodKey)?.description}
        </p>
      ) : null}
      <AdminFieldLabel label="Display name" htmlFor="method-form-display-name" required>
        <Input
          id="method-form-display-name"
          value={displayName}
          onChange={(event) => setDisplayName(event.target.value)}
          placeholder="User-facing name (e.g., Alipay WAP)"
          required
        />
      </AdminFieldLabel>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Status" htmlFor="method-form-status">
          <Select
            value={status}
            onValueChange={(value) => setStatus(value as PaymentEntityStatus)}
          >
            <SelectTrigger id="method-form-status">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {STATUS_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
      </div>
      {mode === "create" ? (
        <AdminFieldLabel label="Scope" htmlFor="method-form-scope">
          <Select
            value={scope}
            onValueChange={(value) => setScope(value as PaymentMethodScope)}
          >
            <SelectTrigger id="method-form-scope">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SCOPE_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
      ) : (
        <p className="text-xs text-[var(--sdk-color-text-muted)]">
          Scope is immutable after creation.
        </p>
      )}
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
        <AdminFieldLabel label="Currency" htmlFor="method-form-currency">
          <Input
            id="method-form-currency"
            value={currencyCode}
            onChange={(event) => setCurrencyCode(event.target.value)}
            placeholder="CNY"
            maxLength={3}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Country" htmlFor="method-form-country">
          <Input
            id="method-form-country"
            value={countryCode}
            onChange={(event) => setCountryCode(event.target.value)}
            placeholder="CN"
            maxLength={2}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Sort order" htmlFor="method-form-sort">
          <Input
            id="method-form-sort"
            type="number"
            value={sortOrder}
            onChange={(event) => setSortOrder(event.target.value)}
            placeholder="0"
          />
        </AdminFieldLabel>
      </div>
      {error ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {error}
        </div>
      ) : null}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="ghost" onClick={props.onCancel}>
          Cancel
        </Button>
        <Button type="submit">
          {mode === "create" ? "Create" : "Save changes"}
        </Button>
      </div>
    </form>
  );
}
