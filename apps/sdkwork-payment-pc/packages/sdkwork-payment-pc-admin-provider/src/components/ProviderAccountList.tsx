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
  PaymentProviderIcon,
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
  canCreate: boolean;
  canEdit: boolean;
  canRotate: boolean;
  canTest: boolean;
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
          {props.canCreate ? <div className="mt-3">
            <Button
              type="button"
              variant="primary"
              size="sm"
              onClick={props.onCreate}
              disabled={props.busy}
            >
              Create provider account
            </Button>
          </div> : null}
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
                <div className="flex min-w-0 flex-1 items-start gap-3">
                  <PaymentProviderIcon
                    label={ADMIN_PROVIDER_LABEL[account.providerCode]}
                    providerCode={account.providerCode}
                    size="md"
                  />
                  <div className="min-w-0 flex-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <span className="font-semibold text-[var(--sdk-color-text-primary)]">
                        {account.accountNo}
                      </span>
                      <Badge variant="outline">{ADMIN_PROVIDER_LABEL[account.providerCode]}</Badge>
                      <Badge variant="secondary">
                        {account.accountMode === "partner" ? "Partner / ISV" : "Direct"}
                      </Badge>
                      <Badge variant="outline">{ENV_LABEL[account.environment]}</Badge>
                      <Badge variant={STATUS_TONE[account.status]}>{STATUS_LABEL[account.status]}</Badge>
                    </div>
                    <div className="mt-2 flex flex-wrap items-center gap-1.5" aria-label="Credential readiness">
                      <Badge variant={account.hasPrimarySecret ? "success" : "warning"}>Primary secret</Badge>
                      <Badge variant={account.hasWebhookSecret ? "success" : "secondary"}>Webhook secret</Badge>
                      <Badge variant={account.hasCertificate ? "success" : "secondary"}>Certificate</Badge>
                      <Badge variant={account.lastTestStatus === "success" ? "success" : account.lastTestStatus === "failure" ? "danger" : "warning"}>
                        {TEST_STATUS_LABEL[account.lastTestStatus ?? "unknown"]}
                      </Badge>
                    </div>
                    <dl className="mt-2 grid grid-cols-1 gap-x-6 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)] md:grid-cols-3">
                      <div>
                        <dt className="inline">Merchant ID:</dt>{" "}
                        <dd className="inline font-mono text-[var(--sdk-color-text-primary)]">{account.merchantId ?? "--"}</dd>
                      </div>
                      <div>
                        <dt className="inline">Settlement:</dt>{" "}
                        <dd className="inline font-medium text-[var(--sdk-color-text-primary)]">
                          {account.settlementCurrency}{account.countryCode ? ` / ${account.countryCode}` : ""}
                        </dd>
                      </div>
                      <div>
                        <dt className="inline">Last test:</dt>{" "}
                        <dd className="inline">{account.lastTestedAt ? formatAdminTimestamp(account.lastTestedAt) : "Run before activation"}</dd>
                      </div>
                    </dl>
                  </div>
                </div>
                <div className="flex flex-wrap items-center justify-end gap-2 sm:self-center">
                  {props.canTest ? <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onTest(account)}
                    disabled={props.busy}
                    title="Cannot test while another operation is in progress"
                  >
                    Test
                  </Button> : null}
                  {props.canRotate ? <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onRotate(account)}
                    disabled={props.busy}
                    title="Cannot rotate while another operation is in progress"
                  >
                    Rotate
                  </Button> : null}
                  {props.canEdit ? <Button
                    type="button"
                    variant="ghost"
                    size="sm"
                    onClick={() => props.onEdit(account)}
                    disabled={props.busy}
                    title="Cannot edit while another operation is in progress"
                  >
                    Edit
                  </Button> : null}
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
