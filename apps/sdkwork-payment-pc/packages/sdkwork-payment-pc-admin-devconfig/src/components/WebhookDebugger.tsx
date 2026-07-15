/**
 * Webhook debugger.
 *
 * Two-panel dev tool aligned with industry PSP CLIs (Stripe CLI `trigger` +
 * `listen`):
 *   1. Sandbox trigger — simulate a PSP webhook event for local/sandbox
 *      integration. Only allowed when the target provider account environment
 *      is `development` or `sandbox`. The `amount` / `currencyCode` /
 *      `outTradeNo` fields override the default sandbox payload template.
 *   2. Signature test — verify a raw payload + signature against the
 *      configured `webhook_secret_ref` of the target provider account.
 *
 * Both operations invoke `service.backend.dev.sandboxTrigger` and
 * `service.backend.dev.webhookSignatureTest` respectively (mirrors OpenAPI
 * `SandboxTriggerCommand` and `WebhookSignatureTestCommand`). Results are
 * surfaced inline with diagnostic details.
 */

import * as React from "react";
import {
  Badge,
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@sdkwork/ui-pc-react";
import {
  ADMIN_PROVIDER_LABEL,
  AdminFieldLabel,
  formatAdminTimestamp,
} from "@sdkwork/payment-pc-admin-core";
import type {
  PaymentDevSandboxTriggerResult,
  PaymentDevWebhookSignatureTestResult,
  PaymentProviderAccountView,
  PaymentWebhookEventView,
} from "../types/devconfig-admin-types";

export interface WebhookDebuggerProps {
  accounts: readonly PaymentProviderAccountView[];
  recentEvents: readonly PaymentWebhookEventView[];
  busy?: boolean;
  lastSandboxTriggerResult?: PaymentDevSandboxTriggerResult;
  lastSignatureTestResult?: PaymentDevWebhookSignatureTestResult;
  onSandboxTrigger(
    providerAccountId: string,
    eventType: string,
    overrides: { amount?: string; currencyCode?: string; outTradeNo?: string },
  ): Promise<void> | void;
  onSignatureTest(
    providerAccountId: string,
    payload: string,
    signature: string,
    timestamp: string,
    signatureHeader: string,
  ): Promise<void> | void;
}

const SANDBOX_EVENT_PRESETS: ReadonlyArray<{ label: string; value: string; provider: PaymentProviderAccountView["providerCode"] }> = [
  { label: "Stripe: payment_intent.succeeded", value: "payment_intent.succeeded", provider: "stripe" },
  { label: "Stripe: payment_intent.payment_failed", value: "payment_intent.payment_failed", provider: "stripe" },
  { label: "Stripe: charge.refunded", value: "charge.refunded", provider: "stripe" },
  { label: "Alipay: trade.success", value: "trade.success", provider: "alipay" },
  { label: "Alipay: trade.close", value: "trade.close", provider: "alipay" },
  { label: "WeChat Pay: pay.transaction.success", value: "pay.transaction.success", provider: "wechat_pay" },
  { label: "WeChat Pay: pay.transaction.error", value: "pay.transaction.error", provider: "wechat_pay" },
];

const STATUS_VARIANT: Record<PaymentWebhookEventView["status"], "secondary" | "success" | "danger" | "warning"> = {
  queued: "secondary",
  processing: "warning",
  processed: "success",
  failed: "danger",
  dead: "secondary",
};

export function WebhookDebugger(props: WebhookDebuggerProps) {
  const eligibleAccounts = props.accounts.filter(
    (account) => account.environment === "development" || account.environment === "sandbox",
  );

  return (
    <div className="space-y-6" data-slot="webhook-debugger">
      <SandboxTriggerPanel
        accounts={eligibleAccounts}
        busy={props.busy}
        lastResult={props.lastSandboxTriggerResult}
        onTrigger={props.onSandboxTrigger}
      />
      <SignatureTestPanel
        accounts={props.accounts}
        busy={props.busy}
        lastResult={props.lastSignatureTestResult}
        onTest={props.onSignatureTest}
      />
      <RecentEventsPanel events={props.recentEvents} />
    </div>
  );
}

interface SandboxTriggerPanelProps {
  accounts: readonly PaymentProviderAccountView[];
  busy?: boolean;
  lastResult?: PaymentDevSandboxTriggerResult;
  onTrigger(
    providerAccountId: string,
    eventType: string,
    overrides: { amount?: string; currencyCode?: string; outTradeNo?: string },
  ): Promise<void> | void;
}

function SandboxTriggerPanel(props: SandboxTriggerPanelProps) {
  const [providerAccountId, setProviderAccountId] = React.useState("");
  const [eventType, setEventType] = React.useState("");
  const [amount, setAmount] = React.useState("");
  const [currencyCode, setCurrencyCode] = React.useState("");
  const [outTradeNo, setOutTradeNo] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();

  React.useEffect(() => {
    if (!providerAccountId && props.accounts.length > 0) {
      setProviderAccountId(props.accounts[0].id);
    }
  }, [props.accounts, providerAccountId]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!providerAccountId) {
      setError("Select a provider account first.");
      return;
    }
    if (!eventType.trim()) {
      setError("Event type is required.");
      return;
    }
    try {
      await props.onTrigger(providerAccountId, eventType.trim(), {
        ...(amount.trim() ? { amount: amount.trim() } : {}),
        ...(currencyCode.trim() ? { currencyCode: currencyCode.trim() } : {}),
        ...(outTradeNo.trim() ? { outTradeNo: outTradeNo.trim() } : {}),
      });
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to trigger sandbox event.");
    }
  }

  return (
    <section className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
      <header className="mb-3">
        <div className="text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Sandbox trigger
        </div>
        <p className="mt-1 text-xs text-[var(--sdk-color-text-secondary)]">
          Simulate a PSP webhook event for local/sandbox integration. Mirrors Stripe CLI
          <code className="mx-1 rounded bg-[var(--sdk-color-bg-subtle)] px-1 text-xs">stripe trigger</code>.
          Only development/sandbox accounts are eligible. Leave overrides blank to use the
          provider&apos;s default sandbox payload template.
        </p>
      </header>
      <form className="space-y-3" onSubmit={handleSubmit}>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <AdminFieldLabel label="Provider account" htmlFor="webhook-debugger-trigger-account" required>
            <Select
              value={providerAccountId}
              onValueChange={setProviderAccountId}
              disabled={props.busy}
            >
              <SelectTrigger id="webhook-debugger-trigger-account">
                <SelectValue placeholder={props.accounts.length === 0 ? "No eligible accounts" : "Select account..."} />
              </SelectTrigger>
              <SelectContent>
                {props.accounts.map((account) => (
                  <SelectItem key={account.id} value={account.id}>
                    {account.accountNo} ({ADMIN_PROVIDER_LABEL[account.providerCode]} · {account.environment})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </AdminFieldLabel>
          <AdminFieldLabel label="Event type" htmlFor="webhook-debugger-trigger-event-type" required>
            <Input
              id="webhook-debugger-trigger-event-type"
              value={eventType}
              onChange={(event) => setEventType(event.target.value)}
              placeholder="e.g., payment_intent.succeeded"
              disabled={props.busy}
              required
            />
          </AdminFieldLabel>
        </div>
        <div className="flex flex-wrap gap-2">
          {SANDBOX_EVENT_PRESETS.map((preset) => (
            <Badge
              key={preset.value}
              variant="outline"
              className="cursor-pointer hover:bg-[var(--sdk-color-bg-subtle)]"
              onClick={() => setEventType(preset.value)}
            >
              {preset.label}
            </Badge>
          ))}
        </div>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <AdminFieldLabel label="Amount (optional)" htmlFor="webhook-debugger-trigger-amount">
            <Input
              id="webhook-debugger-trigger-amount"
              value={amount}
              onChange={(event) => setAmount(event.target.value)}
              placeholder="e.g., 10.00"
              disabled={props.busy}
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Currency (optional)" htmlFor="webhook-debugger-trigger-currency">
            <Input
              id="webhook-debugger-trigger-currency"
              value={currencyCode}
              onChange={(event) => setCurrencyCode(event.target.value)}
              placeholder="e.g., USD"
              disabled={props.busy}
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Out trade no (optional)" htmlFor="webhook-debugger-trigger-out-trade-no">
            <Input
              id="webhook-debugger-trigger-out-trade-no"
              value={outTradeNo}
              onChange={(event) => setOutTradeNo(event.target.value)}
              placeholder="Existing attempt out_trade_no"
              disabled={props.busy}
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
        {props.lastResult ? (
          <div
            role="status"
            className={
              "rounded-md border p-3 text-sm " +
              (props.lastResult.ok
                ? "border-[var(--sdk-color-border-success)] bg-[var(--sdk-color-bg-success-subtle)] text-[var(--sdk-color-text-success)]"
                : "border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] text-[var(--sdk-color-text-error)]")
            }
          >
            <div className="font-medium">
              {props.lastResult.ok ? "Sandbox event triggered" : "Sandbox trigger failed"}
            </div>
            <div className="mt-1 text-xs">
              Provider: {props.lastResult.providerCode} · Environment: {props.lastResult.environment}
              {props.lastResult.operationId ? ` · Operation: ${props.lastResult.operationId}` : ""}
              {props.lastResult.status ? ` · Status: ${props.lastResult.status}` : ""}
              {props.lastResult.diagnostic ? ` · ${props.lastResult.diagnostic}` : ""}
            </div>
          </div>
        ) : null}
        <div className="flex justify-end">
          <Button type="submit" disabled={props.busy || props.accounts.length === 0}>
            {props.busy ? "Triggering..." : "Trigger sandbox event"}
          </Button>
        </div>
      </form>
    </section>
  );
}

interface SignatureTestPanelProps {
  accounts: readonly PaymentProviderAccountView[];
  busy?: boolean;
  lastResult?: PaymentDevWebhookSignatureTestResult;
  onTest(
    providerAccountId: string,
    payload: string,
    signature: string,
    timestamp: string,
    signatureHeader: string,
  ): Promise<void> | void;
}

function SignatureTestPanel(props: SignatureTestPanelProps) {
  const [providerAccountId, setProviderAccountId] = React.useState("");
  const [payload, setPayload] = React.useState("");
  const [signature, setSignature] = React.useState("");
  const [timestamp, setTimestamp] = React.useState("");
  const [signatureHeader, setSignatureHeader] = React.useState("");
  const [error, setError] = React.useState<string | undefined>();

  React.useEffect(() => {
    if (!providerAccountId && props.accounts.length > 0) {
      setProviderAccountId(props.accounts[0].id);
    }
  }, [props.accounts, providerAccountId]);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!providerAccountId) {
      setError("Select a provider account first.");
      return;
    }
    if (!payload.trim()) {
      setError("Raw payload is required.");
      return;
    }
    if (!signature.trim()) {
      setError("Signature is required.");
      return;
    }
    try {
      await props.onTest(providerAccountId, payload, signature.trim(), timestamp.trim(), signatureHeader.trim());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to test webhook signature.");
    }
  }

  return (
    <section className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
      <header className="mb-3">
        <div className="text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Webhook signature test
        </div>
        <p className="mt-1 text-xs text-[var(--sdk-color-text-secondary)]">
          Verify a raw payload + signature against the configured
          <code className="mx-1 rounded bg-[var(--sdk-color-bg-subtle)] px-1 text-xs">webhook_secret_ref</code>
          of the target provider account. Mirrors Stripe CLI
          <code className="mx-1 rounded bg-[var(--sdk-color-bg-subtle)] px-1 text-xs">stripe listen --verify</code>.
        </p>
      </header>
      <form className="space-y-3" onSubmit={handleSubmit}>
        <AdminFieldLabel label="Provider account" htmlFor="webhook-debugger-sig-account" required>
          <Select
            value={providerAccountId}
            onValueChange={setProviderAccountId}
            disabled={props.busy}
          >
            <SelectTrigger id="webhook-debugger-sig-account">
              <SelectValue placeholder="Select account..." />
            </SelectTrigger>
            <SelectContent>
              {props.accounts.map((account) => (
                <SelectItem key={account.id} value={account.id}>
                  {account.accountNo} ({ADMIN_PROVIDER_LABEL[account.providerCode]} · {account.environment})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Raw payload" htmlFor="webhook-debugger-sig-payload" required>
          <textarea
            id="webhook-debugger-sig-payload"
            className="min-h-[8rem] w-full rounded-md border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 py-2 font-mono text-sm shadow-[var(--sdk-shadow-sm)] outline-none focus:ring-2 focus:ring-[var(--sdk-color-border-focus)]"
            value={payload}
            onChange={(event) => setPayload(event.target.value)}
            placeholder="Paste the raw webhook request body..."
            disabled={props.busy}
            required
          />
        </AdminFieldLabel>
        <div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
          <AdminFieldLabel label="Signature" htmlFor="webhook-debugger-sig-signature" required>
            <Input
              id="webhook-debugger-sig-signature"
              value={signature}
              onChange={(event) => setSignature(event.target.value)}
              placeholder="e.g., t=...,v1=..."
              disabled={props.busy}
              required
            />
          </AdminFieldLabel>
          <AdminFieldLabel label="Timestamp (optional)" htmlFor="webhook-debugger-sig-timestamp">
            <Input
              id="webhook-debugger-sig-timestamp"
              value={timestamp}
              onChange={(event) => setTimestamp(event.target.value)}
              placeholder="Unix timestamp (for replay protection)"
              disabled={props.busy}
            />
          </AdminFieldLabel>
        </div>
        <AdminFieldLabel label="Signature header name override (optional)" htmlFor="webhook-debugger-sig-header-name">
          <Input
            id="webhook-debugger-sig-header-name"
            value={signatureHeader}
            onChange={(event) => setSignatureHeader(event.target.value)}
            placeholder="e.g., Wechatpay-Signature (for non-standard headers)"
            disabled={props.busy}
          />
        </AdminFieldLabel>
        {error ? (
          <div
            role="alert"
            className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
          >
            {error}
          </div>
        ) : null}
        {props.lastResult ? (
          <div
            role="status"
            className={
              "rounded-md border p-3 text-sm " +
              (props.lastResult.ok
                ? "border-[var(--sdk-color-border-success)] bg-[var(--sdk-color-bg-success-subtle)] text-[var(--sdk-color-text-success)]"
                : "border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] text-[var(--sdk-color-text-error)]")
            }
          >
            <div className="font-medium">
              {props.lastResult.ok ? "Signature verified" : "Signature verification failed"}
            </div>
            <div className="mt-1 text-xs">
              Provider: {props.lastResult.providerCode}
              {props.lastResult.algorithm ? ` · Algorithm: ${props.lastResult.algorithm}` : ""}
              {props.lastResult.diagnostic ? ` · ${props.lastResult.diagnostic}` : ""}
              {` · Tested at: ${formatAdminTimestamp(props.lastResult.testedAt)}`}
            </div>
          </div>
        ) : null}
        <div className="flex justify-end">
          <Button type="submit" disabled={props.busy}>
            {props.busy ? "Verifying..." : "Verify signature"}
          </Button>
        </div>
      </form>
    </section>
  );
}

interface RecentEventsPanelProps {
  events: readonly PaymentWebhookEventView[];
}

function RecentEventsPanel(props: RecentEventsPanelProps) {
  if (props.events.length === 0) {
    return null;
  }
  return (
    <section className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
      <header className="mb-3">
        <div className="text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Recent webhook events
        </div>
        <p className="mt-1 text-xs text-[var(--sdk-color-text-secondary)]">
          The latest 5 webhook events received by this tenant. View the Integration Logs tab for the full list and replay.
        </p>
      </header>
      <ul className="divide-y divide-[var(--sdk-color-border-subtle)]">
        {props.events.slice(0, 5).map((event) => (
          <li key={event.id} className="py-2 text-xs">
            <div className="flex flex-wrap items-center gap-2">
              <Badge variant="outline">{event.providerCode}</Badge>
              <span className="font-mono">{event.eventType}</span>
              <Badge variant={STATUS_VARIANT[event.status]}>{event.status}</Badge>
              <span className="text-[var(--sdk-color-text-secondary)]">
                {formatAdminTimestamp(event.receivedAt)}
              </span>
            </div>
          </li>
        ))}
      </ul>
    </section>
  );
}
