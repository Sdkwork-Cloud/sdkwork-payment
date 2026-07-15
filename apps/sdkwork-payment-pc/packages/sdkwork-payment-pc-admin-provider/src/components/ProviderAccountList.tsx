/**
 * Provider account list with status badges, environment/mode indicators, and
 * credential test/rotate action shortcuts. Designed for the admin workspace.
 */

import * as React from "react";
import { Badge, Button } from "@sdkwork/ui-pc-react";
import {
  SdkworkPaymentListPaginationControls,
  ADMIN_PROVIDER_LABEL,
  formatAdminTimestamp,
} from "@sdkwork/payment-pc-admin-core";
import type {
  PaymentProviderAccountView,
  PaymentLastTestStatus,
} from "../types/provider-admin-types";

export interface ProviderAccountListProps {
  accounts: readonly PaymentProviderAccountView[];
  pageInfo?: import("@sdkwork/payment-contracts").SdkWorkPageInfo;
  selectedId?: string;
  busy?: boolean;
  onSelect(account: PaymentProviderAccountView): void;
  onEdit(account: PaymentProviderAccountView): void;
  onTest(account: PaymentProviderAccountView): void;
  onRotate(account: PaymentProviderAccountView): void;
  // Empty-state inline create button callback; parent component wires it to the create dialog
  onCreate(): void;
  onLoadMore(): void;
}

const STATUS_LABEL: Record<PaymentProviderAccountView["status"], string> = {
  active: "Active",
  inactive: "Inactive",
  suspended: "Suspended",
  deprecated: "Deprecated",
};

const STATUS_TONE: Record<
  PaymentProviderAccountView["status"],
  "success" | "secondary" | "warning" | "danger"
> = {
  active: "success",
  inactive: "secondary",
  suspended: "warning",
  deprecated: "danger",
};

const ENV_LABEL: Record<PaymentProviderAccountView["environment"], string> = {
  development: "Dev",
  sandbox: "Sandbox",
  production: "Prod",
};

const TEST_STATUS_LABEL: Record<PaymentLastTestStatus, string> = {
  success: "Healthy",
  failure: "Failed",
  unknown: "Untested",
};

export function ProviderAccountList(props: ProviderAccountListProps) {
  return (
    <div className="space-y-3" data-slot="provider-account-list">
      {props.accounts.length === 0 ? (
        <div className="rounded-md border border-dashed border-[var(--sdk-color-border-subtle)] p-8 text-center text-sm text-[var(--sdk-color-text-secondary)]">
          No provider accounts yet. Create one to configure payment channels.
          {/* Empty-state inline create button: guides users to create a provider account directly */}
          <div className="mt-3">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={props.onCreate}
              disabled={props.busy}
            >
              Create provider account
            </Button>
          </div>
        </div>
      ) : (
        <ul className="divide-y divide-[var(--sdk-color-border-subtle)] rounded-md border border-[var(--sdk-color-border-subtle)]">
          {props.accounts.map((account) => {
            const isSelected = props.selectedId === account.id;
            return (
              <li
                key={account.id}
                aria-current={isSelected ? "true" : undefined}
                className={
                  "flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between " +
                  (isSelected ? "bg-[var(--sdk-color-bg-subtle)]" : "")
                }
                data-slot="provider-account-row"
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
                    <Badge variant="outline">{ENV_LABEL[account.environment]}</Badge>
                    <Badge variant={STATUS_TONE[account.status]}>
                      {STATUS_LABEL[account.status]}
                    </Badge>
                  </div>
                  <dl className="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] sm:grid-cols-3">
                    <div>
                      <dt className="inline">Merchant ID:</dt>{" "}
                      <dd className="inline">{account.merchantId ?? "—"}</dd>
                    </div>
                    <div>
                      <dt className="inline">Currency:</dt>{" "}
                      <dd className="inline">
                        {account.settlementCurrency}
                        {account.countryCode ? ` / ${account.countryCode}` : ""}
                      </dd>
                    </div>
                    <div>
                      <dt className="inline">Last test:</dt>{" "}
                      <dd className="inline">
                        {account.lastTestedAt
                          ? `${TEST_STATUS_LABEL[account.lastTestStatus ?? "unknown"]} · ${formatAdminTimestamp(account.lastTestedAt)}`
                          : "Untested"}
                      </dd>
                    </div>
                  </dl>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onTest(account)}
                    disabled={props.busy}
                    title="Cannot test while another operation is in progress"
                  >
                    Test
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onRotate(account)}
                    disabled={props.busy}
                    title="Cannot rotate while another operation is in progress"
                  >
                    Rotate
                  </Button>
                  <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onEdit(account)}
                    disabled={props.busy}
                    title="Cannot edit while another operation is in progress"
                  >
                    Edit
                  </Button>
                  <Button
                    type="button"
                    size="sm"
                    onClick={() => props.onSelect(account)}
                    disabled={props.busy}
                    title="Cannot select while another operation is in progress"
                  >
                    {isSelected ? "Selected" : "Select"}
                  </Button>
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
    </div>
  );
}
