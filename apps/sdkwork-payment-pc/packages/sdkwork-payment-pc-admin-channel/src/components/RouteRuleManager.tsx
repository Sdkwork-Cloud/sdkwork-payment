/**
 * Route rule manager.
 *
 * Full CRUD for routing rules. A route rule is the "traffic controller" of
 * payments — given a set of match conditions (purchase type, country, currency,
 * client platform, amount range, user segment, risk level), it routes the
 * payment to a specific channel (which in turn determines the provider account).
 *
 * API matrix: list + create + update + delete (no retrieve — detail loaded
 * from list items cache).
 *
 * Match conditions are flat fields on the rule (not a separate schema per
 * OpenAPI). The form groups them visually (Conditions / Action / Time window)
 * but the wire payload is a single object.
 *
 * Mirrors industry PSP routing rule surfaces (Stripe Dashboard → Routing rules,
 * Adyen → Payment routing, WeChat Pay → payment scene routing).
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
  ConfirmDialog,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentChannelView,
  PaymentEntityStatus,
  PaymentRouteRuleDraft,
  PaymentRouteRuleUpdateDraft,
  PaymentRouteRuleView,
} from "../types/channel-admin-types";

export interface RouteRuleManagerProps {
  routeRules: readonly PaymentRouteRuleView[];
  channels: readonly PaymentChannelView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  canCreate: boolean;
  canDelete: boolean;
  canUpdate: boolean;
  onCreate(draft: PaymentRouteRuleDraft): Promise<void> | void;
  onUpdate(id: string, draft: PaymentRouteRuleUpdateDraft): Promise<void> | void;
  onDelete(id: string): Promise<void> | void;
  onLoadMore(): void;
}

const STATUS_VARIANT: Record<PaymentEntityStatus, "success" | "secondary" | "danger"> = {
  active: "success",
  inactive: "secondary",
  deprecated: "danger",
};

const STATUS_OPTIONS: ReadonlyArray<{ label: string; value: PaymentEntityStatus }> = [
  { label: "Active", value: "active" },
  { label: "Inactive", value: "inactive" },
  { label: "Deprecated", value: "deprecated" },
];

export function RouteRuleManager(props: RouteRuleManagerProps) {
  const [dialog, setDialog] = React.useState<
    | { kind: "closed" }
    | { kind: "create" }
    | { kind: "edit"; rule: PaymentRouteRuleView }
  >({ kind: "closed" });
  const [pendingDelete, setPendingDelete] = React.useState<PaymentRouteRuleView | null>(null);
  const [error, setError] = React.useState<string | undefined>();

  async function handleCreate(draft: PaymentRouteRuleDraft) {
    await props.onCreate(draft);
    setDialog({ kind: "closed" });
  }

  async function handleUpdate(draft: PaymentRouteRuleUpdateDraft) {
    if (dialog.kind !== "edit") {
      return;
    }
    await props.onUpdate(dialog.rule.id, draft);
    setDialog({ kind: "closed" });
  }

  async function handleConfirmDelete() {
    if (!pendingDelete) return;
    setError(undefined);
    try {
      await props.onDelete(pendingDelete.id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete route rule.");
    }
    setPendingDelete(null);
  }

  return (
    <div className="space-y-4" data-slot="route-rule-manager">
      <div className="flex justify-end">
        {props.canCreate ? <Button
          type="button"
          size="sm"
          onClick={() => setDialog({ kind: "create" })}
          disabled={props.busy || props.channels.length === 0}
          title={
            props.channels.length === 0
              ? "Create a payment channel first — rules route to channels"
              : "Create a new routing rule"
          }
        >
          Create route rule
        </Button> : null}
      </div>

      {props.routeRules.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No routing rules configured. Without rules, payments will be rejected (no
          matching channel). Create one to start routing payments.
          {/* Empty-state inline create button: disabled when no channels exist, mirroring the header button logic */}
          {props.canCreate ? <div className="mt-3">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={() => setDialog({ kind: "create" })}
              disabled={props.busy || props.channels.length === 0}
            >
              Create route rule
            </Button>
          </div> : null}
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {/* Sort ascending by priority: lower number means higher priority; spread into a new array to avoid mutating props */}
          {[...props.routeRules]
            .sort((a, b) => a.priority - b.priority)
            .map((rule) => {
            const channel = props.channels.find((c) => c.id === rule.channelId);
            return (
              <li key={rule.id} className="space-y-2 p-4">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-[var(--sdk-color-text)]">
                    {rule.ruleNo}
                  </span>
                  <Badge variant="outline">Priority: {rule.priority}</Badge>
                  <Badge variant={STATUS_VARIANT[rule.status]}>{rule.status}</Badge>
                  <span className="text-xs text-[var(--sdk-color-text-muted)]">
                    → {channel ? channel.channelNo : rule.channelId}
                  </span>
                </div>
                <dl className="grid grid-cols-1 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-3">
                  {rule.purchaseType ? (
                    <div>
                      <dt className="inline">Purchase type:</dt>{" "}
                      <dd className="inline">{rule.purchaseType}</dd>
                    </div>
                  ) : null}
                  {rule.countryCode ? (
                    <div>
                      <dt className="inline">Country:</dt>{" "}
                      <dd className="inline">{rule.countryCode}</dd>
                    </div>
                  ) : null}
                  {rule.currencyCode ? (
                    <div>
                      <dt className="inline">Currency:</dt>{" "}
                      <dd className="inline">{rule.currencyCode}</dd>
                    </div>
                  ) : null}
                  {rule.clientPlatform ? (
                    <div>
                      <dt className="inline">Client platform:</dt>{" "}
                      <dd className="inline">{rule.clientPlatform}</dd>
                    </div>
                  ) : null}
                  {rule.amountMin || rule.amountMax ? (
                    <div>
                      <dt className="inline">Amount:</dt>{" "}
                      <dd className="inline">
                        {rule.amountMin ?? "*"} ~ {rule.amountMax ?? "*"}
                      </dd>
                    </div>
                  ) : null}
                  {rule.userSegment ? (
                    <div>
                      <dt className="inline">User segment:</dt>{" "}
                      <dd className="inline">{rule.userSegment}</dd>
                    </div>
                  ) : null}
                  {rule.riskLevel ? (
                    <div>
                      <dt className="inline">Risk level:</dt>{" "}
                      <dd className="inline">{rule.riskLevel}</dd>
                    </div>
                  ) : null}
                </dl>
                {rule.startsAt || rule.endsAt ? (
                  <div className="text-xs text-[var(--sdk-color-text-muted)]">
                    Valid window: {rule.startsAt ?? "now"} → {rule.endsAt ?? "forever"}
                  </div>
                ) : null}
                <div className="flex justify-end gap-2">
                  {props.canUpdate ? <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setDialog({ kind: "edit", rule })}
                    disabled={props.busy}
                    title="Cannot edit while another operation is in progress"
                  >
                    Edit
                  </Button> : null}
                  {props.canDelete ? <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => setPendingDelete(rule)}
                    disabled={props.busy}
                    title="Cannot delete while another operation is in progress"
                  >
                    Delete
                  </Button> : null}
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
              {dialog.kind === "create" ? "Create route rule" : "Edit route rule"}
            </DialogTitle>
          </DialogHeader>
          {dialog.kind === "create" || dialog.kind === "edit" ? (
            <RouteRuleForm
              mode={dialog.kind === "create" ? "create" : "update"}
              initial={dialog.kind === "edit" ? dialog.rule : undefined}
              channels={props.channels}
              onCancel={() => setDialog({ kind: "closed" })}
              onSubmit={
                dialog.kind === "create"
                  ? (draft) => handleCreate(draft as PaymentRouteRuleDraft)
                  : (draft) => handleUpdate(draft as PaymentRouteRuleUpdateDraft)
              }
            />
          ) : null}
        </DialogContent>
      </Dialog>

      {error ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {error}
        </div>
      ) : null}

      <ConfirmDialog
        open={props.canDelete && pendingDelete !== null}
        title="Delete route rule?"
        description={
          pendingDelete
            ? `Delete route rule ${pendingDelete.ruleNo}? This permanently removes the rule. Payments matching this rule's conditions will fall through to the next matching rule (if any).`
            : ""
        }
        confirmLabel="Delete"
        variant="danger"
        busy={props.busy}
        onConfirm={handleConfirmDelete}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null);
        }}
      />
    </div>
  );
}

interface RouteRuleFormProps {
  mode: "create" | "update";
  initial?: PaymentRouteRuleView;
  channels: readonly PaymentChannelView[];
  onCancel(): void;
  onSubmit(draft: PaymentRouteRuleDraft | PaymentRouteRuleUpdateDraft): Promise<void> | void;
}

function RouteRuleForm(props: RouteRuleFormProps) {
  const { mode, initial } = props;
  const [ruleNo, setRuleNo] = React.useState(initial?.ruleNo ?? "");
  const [priority, setPriority] = React.useState(String(initial?.priority ?? 0));
  const [purchaseType, setPurchaseType] = React.useState(initial?.purchaseType ?? "");
  const [countryCode, setCountryCode] = React.useState(initial?.countryCode ?? "");
  const [currencyCode, setCurrencyCode] = React.useState(initial?.currencyCode ?? "");
  const [clientPlatform, setClientPlatform] = React.useState(initial?.clientPlatform ?? "");
  const [amountMin, setAmountMin] = React.useState(initial?.amountMin ?? "");
  const [amountMax, setAmountMax] = React.useState(initial?.amountMax ?? "");
  const [userSegment, setUserSegment] = React.useState(initial?.userSegment ?? "");
  const [riskLevel, setRiskLevel] = React.useState(initial?.riskLevel ?? "");
  const [channelId, setChannelId] = React.useState(initial?.channelId ?? "");
  const [status, setStatus] = React.useState<PaymentEntityStatus>(initial?.status ?? "active");
  const [startsAt, setStartsAt] = React.useState(initial?.startsAt?.slice(0, 16) ?? "");
  const [endsAt, setEndsAt] = React.useState(initial?.endsAt?.slice(0, 16) ?? "");
  const [error, setError] = React.useState<string | undefined>();

  React.useEffect(() => {
    if (!channelId && props.channels.length > 0) {
      setChannelId(props.channels[0].id);
    }
  }, [props.channels, channelId]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (mode === "create" && !ruleNo.trim()) {
      setError("Rule number is required.");
      return;
    }
    if (!channelId) {
      setError("Select a target channel.");
      return;
    }
    const priorityNum = Number.parseInt(priority, 10);
    if (Number.isNaN(priorityNum)) {
      setError("Priority must be an integer.");
      return;
    }
    if ((amountMin && !isAmountValid(amountMin)) || (amountMax && !isAmountValid(amountMax))) {
      setError("Amount fields must match pattern like 100 or 99.99.");
      return;
    }
    const startsAtIso = startsAt ? new Date(startsAt).toISOString() : undefined;
    const endsAtIso = endsAt ? new Date(endsAt).toISOString() : undefined;
    try {
      if (mode === "create") {
        await props.onSubmit({
          ruleNo: ruleNo.trim(),
          priority: priorityNum,
          purchaseType: purchaseType.trim() || undefined,
          countryCode: countryCode.trim() || undefined,
          currencyCode: currencyCode.trim() || undefined,
          clientPlatform: clientPlatform.trim() || undefined,
          amountMin: amountMin.trim() || undefined,
          amountMax: amountMax.trim() || undefined,
          userSegment: userSegment.trim() || undefined,
          riskLevel: riskLevel.trim() || undefined,
          channelId,
          status,
          startsAt: startsAtIso,
          endsAt: endsAtIso,
        } as PaymentRouteRuleDraft);
      } else {
        await props.onSubmit({
          priority: priorityNum,
          purchaseType: purchaseType.trim() || undefined,
          countryCode: countryCode.trim() || undefined,
          currencyCode: currencyCode.trim() || undefined,
          clientPlatform: clientPlatform.trim() || undefined,
          amountMin: amountMin.trim() || undefined,
          amountMax: amountMax.trim() || undefined,
          userSegment: userSegment.trim() || undefined,
          riskLevel: riskLevel.trim() || undefined,
          channelId,
          status,
          startsAt: startsAtIso,
          endsAt: endsAtIso,
        } as PaymentRouteRuleUpdateDraft);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save route rule.");
    }
  }

  return (
    <form className="space-y-4" onSubmit={handleSubmit}>
      {mode === "create" ? (
        <AdminFieldLabel label="Rule number" htmlFor="rule-form-no" required>
          <Input
            id="rule-form-no"
            value={ruleNo}
            onChange={(event) => setRuleNo(event.target.value)}
            placeholder="e.g., rule_alipay_wap_cny_001"
            required
          />
        </AdminFieldLabel>
      ) : (
        <p className="text-xs text-[var(--sdk-color-text-muted)]">
          Rule number is immutable after creation.
        </p>
      )}

      <AdminFieldLabel label="Target channel" htmlFor="rule-form-channel" required>
        <Select value={channelId} onValueChange={setChannelId}>
          <SelectTrigger id="rule-form-channel">
            <SelectValue placeholder="Select channel..." />
          </SelectTrigger>
          <SelectContent>
            {props.channels.map((channel) => (
              <SelectItem key={channel.id} value={channel.id}>
                {channel.channelNo} ({channel.sceneCode} · {channel.currencyCode})
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </AdminFieldLabel>

      <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
        <AdminFieldLabel label="Priority" htmlFor="rule-form-priority">
          <Input
            id="rule-form-priority"
            type="number"
            value={priority}
            onChange={(event) => setPriority(event.target.value)}
            placeholder="0 (lower = higher priority)"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Status" htmlFor="rule-form-status">
          <Select
            value={status}
            onValueChange={(value) => setStatus(value as PaymentEntityStatus)}
          >
            <SelectTrigger id="rule-form-status">
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

      <fieldset className="space-y-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3">
        <legend className="px-1 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Match conditions (all optional — empty = match all)
        </legend>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <AdminFieldLabel label="Purchase type" htmlFor="rule-form-purchase-type">
            <Input
              id="rule-form-purchase-type"
              value={purchaseType}
              onChange={(event) => setPurchaseType(event.target.value)}
              placeholder="e.g., goods, digital, subscription"
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Client platform" htmlFor="rule-form-platform">
            <Input
              id="rule-form-platform"
              value={clientPlatform}
              onChange={(event) => setClientPlatform(event.target.value)}
              placeholder="e.g., ios, android, web, mini_program"
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Country" htmlFor="rule-form-country">
            <Input
              id="rule-form-country"
              value={countryCode}
              onChange={(event) => setCountryCode(event.target.value)}
              placeholder="CN"
              maxLength={2}
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Currency" htmlFor="rule-form-currency">
            <Input
              id="rule-form-currency"
              value={currencyCode}
              onChange={(event) => setCurrencyCode(event.target.value)}
              placeholder="CNY"
              maxLength={3}
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Amount min" htmlFor="rule-form-amount-min">
            <Input
              id="rule-form-amount-min"
              value={amountMin}
              onChange={(event) => setAmountMin(event.target.value)}
              placeholder="0.00"
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Amount max" htmlFor="rule-form-amount-max">
            <Input
              id="rule-form-amount-max"
              value={amountMax}
              onChange={(event) => setAmountMax(event.target.value)}
              placeholder="999999.99"
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="User segment" htmlFor="rule-form-segment">
            <Input
              id="rule-form-segment"
              value={userSegment}
              onChange={(event) => setUserSegment(event.target.value)}
              placeholder="e.g., vip, new_user, enterprise"
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Risk level" htmlFor="rule-form-risk">
            <Input
              id="rule-form-risk"
              value={riskLevel}
              onChange={(event) => setRiskLevel(event.target.value)}
              placeholder="e.g., low, medium, high"
            />
          </AdminFieldLabel>
        </div>
      </fieldset>

      <fieldset className="space-y-3 rounded-md border border-[var(--sdk-color-border-subtle)] p-3">
        <legend className="px-1 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Validity window (optional)
        </legend>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <AdminFieldLabel label="Starts at" htmlFor="rule-form-starts">
            <Input
              id="rule-form-starts"
              type="datetime-local"
              value={startsAt}
              onChange={(event) => setStartsAt(event.target.value)}
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Ends at" htmlFor="rule-form-ends">
            <Input
              id="rule-form-ends"
              type="datetime-local"
              value={endsAt}
              onChange={(event) => setEndsAt(event.target.value)}
            />
          </AdminFieldLabel>
        </div>
      </fieldset>

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
          {mode === "create" ? "Create rule" : "Save changes"}
        </Button>
      </div>
    </form>
  );
}

function isAmountValid(value: string): boolean {
  return /^[0-9]+(\.[0-9]{1,2})?$/.test(value);
}
