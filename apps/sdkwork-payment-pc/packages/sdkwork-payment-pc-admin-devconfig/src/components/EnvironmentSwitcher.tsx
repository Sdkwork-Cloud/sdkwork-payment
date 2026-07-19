/**
 * Environment switcher + credential test panel.
 *
 * Lists all provider accounts with their current environment badge and lets the
 * admin switch environments (development / sandbox / production) inline. Each
 * row also exposes a "Test" button that invokes `providerAccounts.test` to verify
 * credentials via the lowest-cost PSP API. The last test result panel surfaces
 * PSP response code, latency, and diagnostic text.
 *
 * Environment switching is gated by the backend per the OpenAPI spec; production
 * transitions require elevated permissions. The UI shows a styled ConfirmDialog
 * before applying the switch (not native window.confirm).
 *
 * Industry PSP alignment:
 *   - Stripe Dashboard uses "danger" red badge for Live mode
 *   - production env uses "danger" variant here (not "success" green)
 *   - secret refs are masked with reveal toggle + copy button (Stripe pattern)
 */

import * as React from "react";
import {
  Badge,
  Button,
  Label,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@sdkwork/ui-pc-react";
import {
  ADMIN_PROVIDER_LABEL,
  ConfirmDialog,
  SecretRefField,
  formatAdminTimestamp,
  SdkworkPaymentListPaginationControls,
} from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type {
  PaymentProviderAccountTestResult,
  PaymentProviderAccountView,
  PaymentProviderEnvironment,
} from "../types/devconfig-admin-types";

export interface EnvironmentSwitcherProps {
  accounts: readonly PaymentProviderAccountView[];
  pageInfo?: SdkWorkPageInfo;
  busy?: boolean;
  lastTestResult?: PaymentProviderAccountTestResult;
  onSwitchEnvironment(id: string, environment: PaymentProviderEnvironment): Promise<void> | void;
  onTest(id: string): Promise<void> | void;
  onLoadMore(): void;
}

const ENV_LABEL: Record<PaymentProviderEnvironment, string> = {
  development: "Development",
  sandbox: "Sandbox",
  production: "Production",
};

// production uses "danger" (red) to signal high-risk environment,
// mirroring Stripe Dashboard Live mode indicator.
const ENV_VARIANT: Record<PaymentProviderEnvironment, "secondary" | "warning" | "danger"> = {
  development: "secondary",
  sandbox: "warning",
  production: "danger",
};

const LAST_TEST_VARIANT: Record<string, "success" | "danger" | "secondary"> = {
  success: "success",
  failure: "danger",
  unknown: "secondary",
};

const LAST_TEST_LABEL: Record<string, string> = {
  success: "Healthy",
  failure: "Failed",
  unknown: "Untested",
};

interface PendingSwitch {
  accountId: string;
  accountNo: string;
  fromEnv: PaymentProviderEnvironment;
  toEnv: PaymentProviderEnvironment;
}

export function EnvironmentSwitcher(props: EnvironmentSwitcherProps) {
  const [pending, setPending] = React.useState<PendingSwitch | null>(null);

  function handleSelectChange(account: PaymentProviderAccountView, value: string) {
    const nextEnv = value as PaymentProviderEnvironment;
    if (nextEnv === account.environment) return;
    setPending({
      accountId: account.id,
      accountNo: account.accountNo,
      fromEnv: account.environment,
      toEnv: nextEnv,
    });
  }

  async function handleConfirmSwitch() {
    if (!pending) return;
    await props.onSwitchEnvironment(pending.accountId, pending.toEnv);
    setPending(null);
  }

  const switchingToProduction = pending?.toEnv === "production";

  return (
    <div className="space-y-4" data-slot="env-switcher">
      {props.accounts.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No provider accounts available. Create one under the Provider admin tab first.
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.accounts.map((account) => (
            <li
              key={account.id}
              className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between"
              data-slot="env-switcher-row"
            >
              <div className="min-w-0 flex-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium text-[var(--sdk-color-text)]">
                    {account.accountNo}
                  </span>
                  <Badge variant="outline">
                    {ADMIN_PROVIDER_LABEL[account.providerCode]}
                  </Badge>
                  <Badge variant="secondary">
                    {account.accountMode === "partner" ? "Partner / ISV" : "Direct"}
                  </Badge>
                  <Badge variant={ENV_VARIANT[account.environment]} title={`${ENV_LABEL[account.environment]} environment`}>
                    {ENV_LABEL[account.environment]}
                  </Badge>
                  <Badge variant={LAST_TEST_VARIANT[account.lastTestStatus ?? "unknown"]}>
                    {account.lastTestedAt
                      ? `${LAST_TEST_LABEL[account.lastTestStatus ?? "unknown"]} · ${formatAdminTimestamp(account.lastTestedAt)}`
                      : "Untested"}
                  </Badge>
                </div>
                <dl className="mt-2 grid grid-cols-1 gap-x-6 gap-y-2 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-2">
                  <div>
                    <dt className="inline">Merchant ID:</dt>{" "}
                    <dd className="inline">{account.merchantId ?? "—"}</dd>
                  </div>
                  <div>
                    <dt className="inline">Cert expiry:</dt>{" "}
                    <dd className="inline">
                      {account.certificateExpiresAt ? formatAdminTimestamp(account.certificateExpiresAt) : "—"}
                    </dd>
                  </div>
                </dl>
                <div className="mt-2">
                  <SecretRefField
                    label="Secret ref"
                    value={account.hasPrimarySecret ? "Configured" : "Missing"}
                    masked
                    helperText="Credential values are write-only and are not returned by the backend."
                  />
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-2">
                <Label htmlFor={`env-switch-${account.id}`} className="sr-only">
                  Environment for {account.accountNo}
                </Label>
                <Select
                  value={account.environment}
                  onValueChange={(value) => handleSelectChange(account, value)}
                  disabled={props.busy}
                >
                  <SelectTrigger id={`env-switch-${account.id}`} className="w-[10rem]">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="development">Development</SelectItem>
                    <SelectItem value="sandbox">Sandbox</SelectItem>
                    <SelectItem value="production">Production</SelectItem>
                  </SelectContent>
                </Select>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => props.onTest(account.id)}
                  disabled={props.busy}
                  title={props.busy ? "Cannot test while another operation is in progress" : "Test credentials via the lowest-cost PSP API"}
                >
                  Test
                </Button>
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

      {props.lastTestResult ? (
        <div
          role="status"
          className={
            "rounded-md border p-4 text-sm " +
            (props.lastTestResult.ok
              ? "border-[var(--sdk-color-border-success)] bg-[var(--sdk-color-bg-success-subtle)] text-[var(--sdk-color-text-success)]"
              : "border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] text-[var(--sdk-color-text-error)]")
          }
        >
          <div className="font-medium">
            {props.lastTestResult.ok ? "Credentials verified" : "Credential test failed"}
          </div>
          <div className="mt-1 text-xs">
            Provider: {props.lastTestResult.providerCode} · Environment:{" "}
            {props.lastTestResult.environment}
            {typeof props.lastTestResult.pspResponseTimeMs === "number"
              ? ` · Latency: ${props.lastTestResult.pspResponseTimeMs}ms`
              : ""}
            {props.lastTestResult.pspResponseCode
              ? ` · PSP code: ${props.lastTestResult.pspResponseCode}`
              : ""}
            {props.lastTestResult.diagnostic ? ` · ${props.lastTestResult.diagnostic}` : ""}
          </div>
        </div>
      ) : null}

      <ConfirmDialog
        open={pending !== null}
        title={switchingToProduction ? "Switch to Production?" : "Switch environment?"}
        description={
          pending
            ? `Switch ${pending.accountNo} from ${ENV_LABEL[pending.fromEnv]} to ${ENV_LABEL[pending.toEnv]}?` +
              (switchingToProduction
                ? " Production environment handles real money. This action requires elevated backend permissions and will be audited."
                : "")
            : ""
        }
        confirmLabel="Switch"
        variant={switchingToProduction ? "danger" : "warning"}
        busy={props.busy}
        onConfirm={handleConfirmSwitch}
        onOpenChange={(open) => {
          if (!open) setPending(null);
        }}
      />
    </div>
  );
}
