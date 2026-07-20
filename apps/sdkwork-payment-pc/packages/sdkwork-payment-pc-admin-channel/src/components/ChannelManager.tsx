/**
 * Channel manager.
 *
 * Lists payment channels (the bridge between PaymentMethod and ProviderAccount)
 * with create-only capability. Per OpenAPI contract, channels have NO update
 * or delete operation — the UI surfaces this honestly by hiding edit/delete
 * buttons and showing an explanatory note.
 *
 * A channel represents a concrete available payment pathway:
 *   PaymentMethod (what) + ProviderAccount (who) + SceneCode (where) +
 *   Currency + Country = a routable channel.
 *
 * Mirrors industry PSP channel registries (Stripe Dashboard → Payment method
 * availability per merchant, Alipay open platform → app channel binding,
 * WeChat Pay merchant platform → payment scene config).
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
  adminPaymentMethodKeyOption,
  PaymentMethodIcon,
  PaymentProviderIcon,
  PaymentSceneIcon,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentChannelDraft,
  PaymentChannelView,
  PaymentEntityStatus,
  PaymentMethodView,
  PaymentProviderAccountView,
  PaymentProviderCode,
  PaymentSceneCode,
} from "../types/channel-admin-types";

export interface ChannelManagerProps {
  channels: readonly PaymentChannelView[];
  methods: readonly PaymentMethodView[];
  providerAccounts: readonly PaymentProviderAccountView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  canCreate: boolean;
  onCreate(draft: PaymentChannelDraft): Promise<void> | void;
  onLoadMore(): void;
}

const SCENE_LABEL: Record<PaymentSceneCode, string> = {
  app: "App",
  web: "Web",
  mini_program: "Mini Program",
  api: "API",
};

const STATUS_VARIANT: Record<PaymentEntityStatus, "success" | "secondary" | "danger"> = {
  active: "success",
  inactive: "secondary",
  deprecated: "danger",
};

const SCENE_OPTIONS: ReadonlyArray<{ label: string; value: PaymentSceneCode }> = [
  { label: "App", value: "app" },
  { label: "Web", value: "web" },
  { label: "Mini Program", value: "mini_program" },
  { label: "API", value: "api" },
];

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: PaymentEntityStatus }> = [
  { label: "Active", value: "active" },
  { label: "Inactive", value: "inactive" },
  { label: "Deprecated", value: "deprecated" },
];

export function ChannelManager(props: ChannelManagerProps) {
  const [open, setOpen] = React.useState(false);

  return (
    <div className="space-y-4" data-slot="channel-manager">
      <div className="flex items-center justify-between">
        <p className="text-xs text-[var(--sdk-color-text-muted)]">
          Channels link a payment method to a provider account under a specific scene.
          Once created, channels cannot be edited or deleted via the API — set the
          status carefully at creation time.
        </p>
        {props.canCreate ? <Button
          type="button"
          size="sm"
          onClick={() => setOpen(true)}
          disabled={props.busy || props.methods.length === 0 || props.providerAccounts.length === 0}
          title={
            props.methods.length === 0 || props.providerAccounts.length === 0
              ? "Create a payment method and provider account first"
              : "Create a new payment channel"
          }
        >
          Create channel
        </Button> : null}
      </div>

      {props.channels.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No payment channels configured. Create one to bridge a payment method with a
          provider account.
          {/* Empty-state inline create button: disabled when a payment method or provider account is missing, mirroring the header button logic */}
          {props.canCreate ? <div className="mt-3">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={() => setOpen(true)}
              disabled={props.busy || props.methods.length === 0 || props.providerAccounts.length === 0}
            >
              Create channel
            </Button>
          </div> : null}
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.channels.map((channel) => {
            const method = props.methods.find((m) => m.id === channel.methodId);
            const providerAccount = props.providerAccounts.find((p) => p.id === channel.providerAccountId);
            return (
              <li key={channel.id} className="flex flex-col gap-3 p-4 xl:flex-row xl:items-center xl:justify-between">
                <div className="flex min-w-0 flex-1 items-start gap-3">
                  <div className="flex shrink-0 items-center -space-x-1.5" aria-label="Payment method to provider route">
                    <PaymentMethodIcon
                      label={method ? adminPaymentMethodKeyOption(method.methodKey)?.label ?? method.displayName : "Payment method"}
                      methodKey={method?.methodKey ?? channel.methodId}
                      providerCode={method?.providerCode ?? channel.providerCode}
                      size="md"
                    />
                    <PaymentProviderIcon
                      className="ring-2 ring-[var(--sdk-color-surface-panel)]"
                      label={providerAccount?.providerCode ?? channel.providerCode ?? "Payment provider"}
                      providerCode={providerAccount?.providerCode ?? channel.providerCode ?? "unknown"}
                      size="md"
                    />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold text-[var(--sdk-color-text-primary)]">
                        {channel.channelName ?? channel.channelNo}
                      </span>
                      <Badge variant="outline" className="font-mono">{channel.channelNo}</Badge>
                      <Badge variant={STATUS_VARIANT[channel.status]}>{channel.status}</Badge>
                      <Badge variant="outline">Priority {channel.priority}</Badge>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-2 text-xs text-[var(--sdk-color-text-secondary)]">
                      <span className="font-medium text-[var(--sdk-color-text-primary)]">
                        {method ? method.displayName : channel.methodId}
                      </span>
                      <span aria-hidden="true" className="text-[var(--sdk-color-text-muted)]">→</span>
                      <span className="font-mono">
                        {providerAccount ? providerAccount.accountNo : channel.providerAccountId}
                      </span>
                    </div>
                  </div>
                </div>
                <div className="flex flex-wrap items-center gap-2 xl:justify-end">
                  <span className="inline-flex items-center gap-1.5 text-xs text-[var(--sdk-color-text-secondary)]">
                    <PaymentSceneIcon sceneCode={channel.sceneCode} size="xs" />
                    {SCENE_LABEL[channel.sceneCode]}
                  </span>
                  <Badge variant="outline">{channel.currencyCode} · {channel.countryCode || "Global"}</Badge>
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <SdkworkPaymentListPaginationControls
        busy={props.busy ?? false}
        onLoadMore={props.onLoadMore}
        pageInfo={props.pageInfo}
      />

      <Dialog
        open={props.canCreate && open}
        onOpenChange={setOpen}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Create payment channel</DialogTitle>
          </DialogHeader>
          <ChannelForm
            methods={props.methods}
            providerAccounts={props.providerAccounts}
            onCancel={() => setOpen(false)}
            onSubmit={async (draft) => {
              await props.onCreate(draft);
              setOpen(false);
            }}
          />
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface ChannelFormProps {
  methods: readonly PaymentMethodView[];
  providerAccounts: readonly PaymentProviderAccountView[];
  onCancel(): void;
  onSubmit(draft: PaymentChannelDraft): Promise<void> | void;
}

function ChannelForm(props: ChannelFormProps) {
  const [channelNo, setChannelNo] = React.useState("");
  const [channelName, setChannelName] = React.useState("");
  const [methodId, setMethodId] = React.useState("");
  const [providerAccountId, setProviderAccountId] = React.useState("");
  const [sceneCode, setSceneCode] = React.useState<PaymentSceneCode>("api");
  const [currencyCode, setCurrencyCode] = React.useState("CNY");
  const [countryCode, setCountryCode] = React.useState("CN");
  const [status, setStatus] = React.useState<PaymentEntityStatus>("active");
  const [priority, setPriority] = React.useState("0");
  const [sortOrder, setSortOrder] = React.useState("0");
  const [error, setError] = React.useState<string | undefined>();

  React.useEffect(() => {
    if (!methodId && props.methods.length > 0) {
      setMethodId(props.methods[0].id);
    }
  }, [props.methods, methodId]);

  React.useEffect(() => {
    if (!providerAccountId && props.providerAccounts.length > 0) {
      setProviderAccountId(props.providerAccounts[0].id);
    }
  }, [props.providerAccounts, providerAccountId]);

  // Auto-fill currencyCode and countryCode from selected method when method changes.
  React.useEffect(() => {
    const method = props.methods.find((m) => m.id === methodId);
    if (method) {
      setCurrencyCode(method.currencyCode);
      if (method.countryCode) {
        setCountryCode(method.countryCode);
      }
    }
  }, [methodId, props.methods]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!channelNo.trim()) {
      setError("Channel number is required.");
      return;
    }
    if (!methodId) {
      setError("Select a payment method.");
      return;
    }
    if (!providerAccountId) {
      setError("Select a provider account.");
      return;
    }
    const priorityNum = Number.parseInt(priority, 10);
    const sortOrderNum = Number.parseInt(sortOrder, 10);
    if (Number.isNaN(priorityNum) || Number.isNaN(sortOrderNum)) {
      setError("Priority and sort order must be integers.");
      return;
    }
    const method = props.methods.find((m) => m.id === methodId);
    const providerAccount = props.providerAccounts.find((p) => p.id === providerAccountId);
    const draft: PaymentChannelDraft = {
      channelNo: channelNo.trim(),
      channelName: channelName.trim() || undefined,
      providerAccountId,
      methodId,
      providerCode: (method?.providerCode ?? providerAccount?.providerCode) as PaymentProviderCode | undefined,
      sceneCode,
      currencyCode: currencyCode.trim() || "CNY",
      countryCode: countryCode.trim(),
      status,
      priority: priorityNum,
      sortOrder: sortOrderNum,
    };
    try {
      await props.onSubmit(draft);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create channel.");
    }
  }

  return (
    <form className="space-y-3" onSubmit={handleSubmit}>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Channel number" htmlFor="channel-form-no" required>
          <Input
            id="channel-form-no"
            value={channelNo}
            onChange={(event) => setChannelNo(event.target.value)}
            placeholder="e.g., alipay_wap_prod_001"
            required
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Channel name (optional)" htmlFor="channel-form-name">
          <Input
            id="channel-form-name"
            value={channelName}
            onChange={(event) => setChannelName(event.target.value)}
            placeholder="Display name"
          />
        </AdminFieldLabel>
      </div>
      <AdminFieldLabel label="Payment method" htmlFor="channel-form-method" required>
        <Select value={methodId} onValueChange={setMethodId}>
          <SelectTrigger id="channel-form-method">
            <SelectValue placeholder="Select method..." />
          </SelectTrigger>
          <SelectContent>
            {props.methods.map((method) => (
              <SelectItem key={method.id} value={method.id}>
                {method.displayName} ({method.methodKey})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminFieldLabel>
      <AdminFieldLabel label="Provider account" htmlFor="channel-form-provider" required>
        <Select value={providerAccountId} onValueChange={setProviderAccountId}>
          <SelectTrigger id="channel-form-provider">
            <SelectValue placeholder="Select provider account..." />
          </SelectTrigger>
          <SelectContent>
            {props.providerAccounts.map((account) => (
              <SelectItem key={account.id} value={account.id}>
                {account.accountNo} ({account.providerCode} · {account.environment})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminFieldLabel>
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Scene" htmlFor="channel-form-scene">
          <Select
            value={sceneCode}
            onValueChange={(value) => setSceneCode(value as PaymentSceneCode)}
          >
            <SelectTrigger id="channel-form-scene">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {SCENE_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Status" htmlFor="channel-form-status">
          <Select
            value={status}
            onValueChange={(value) => setStatus(value as PaymentEntityStatus)}
          >
            <SelectTrigger id="channel-form-status">
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
      <div className="grid grid-cols-1 gap-3 sm:grid-cols-4">
        <AdminFieldLabel label="Currency" htmlFor="channel-form-currency">
          <Input
            id="channel-form-currency"
            value={currencyCode}
            onChange={(event) => setCurrencyCode(event.target.value)}
            placeholder="CNY"
            maxLength={3}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Country" htmlFor="channel-form-country">
          <Input
            id="channel-form-country"
            value={countryCode}
            onChange={(event) => setCountryCode(event.target.value)}
            placeholder="CN"
            maxLength={2}
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Priority" htmlFor="channel-form-priority">
          <Input
            id="channel-form-priority"
            type="number"
            value={priority}
            onChange={(event) => setPriority(event.target.value)}
            placeholder="0"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Sort order" htmlFor="channel-form-sort">
          <Input
            id="channel-form-sort"
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
        <Button type="submit">Create channel</Button>
      </div>
    </form>
  );
}
