/**
 * Provider account dynamic form.
 *
 * Renders different credential fields based on:
 *   1. providerCode (stripe / alipay / wechat_pay / sandbox)
 *   2. accountMode (direct / partner)
 *
 * Direct mode fields per provider:
 *   - stripe: secretRef (env var holding sk_live_... / sk_test_...), webhookSecretRef
 *   - alipay: secretRef (merchantPrivateKey PEM), certificateRef (alipayPublicKey PEM),
 *             metadata.appId, metadata.signType (RSA2/RSA)
 *   - wechat_pay: secretRef (API v3 key), webhookSecretRef (API v3 key),
 *                 certificateRef (platform cert PEM), metadata.merchantSerialNo
 *   - sandbox: secretRef only
 *
 * Partner (ISV) mode fields:
 *   - All direct fields PLUS:
 *   - partnerProviderAccountId (select from existing partner accounts)
 *   - Sub-merchant management is delegated to <SubMerchantManager/>
 *
 * Env var indirection: secrets are NEVER stored as plaintext. The form collects
 * env var NAMES (e.g., STRIPE_SECRET_KEY, ALIPAY_MERCHANT_PRIVATE_KEY) which the
 * backend resolves at runtime via sdkwork-payment-providers::credentials.
 */

import * as React from "react";
import {
  Button,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  Switch,
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  ADMIN_PROVIDER_FORM_OPTIONS,
} from "@sdkwork/payment-pc-admin-core";
import type {
  PaymentProviderAccountDraft,
  PaymentProviderAccountMode,
  PaymentProviderAccountUpdateDraft,
  PaymentProviderAccountView,
  PaymentProviderCode,
  PaymentProviderEnvironment,
  PaymentProviderAccountStatus,
} from "../types/provider-admin-types";

const ACCOUNT_MODE_OPTIONS: readonly { label: string; value: PaymentProviderAccountMode }[] = [
  { label: "Direct (merchant self-connection)", value: "direct" },
  { label: "Partner / ISV (with sub-merchants)", value: "partner" },
];

const ENVIRONMENT_OPTIONS: readonly { label: string; value: PaymentProviderEnvironment }[] = [
  { label: "Development", value: "development" },
  { label: "Sandbox", value: "sandbox" },
  { label: "Production", value: "production" },
];

const STATUS_OPTIONS: readonly { label: string; value: PaymentProviderAccountStatus }[] = [
  { label: "Active", value: "active" },
  { label: "Inactive", value: "inactive" },
  { label: "Suspended", value: "suspended" },
  { label: "Deprecated", value: "deprecated" },
];

const CAPABILITY_KEYS = ["pay", "refund", "close", "query", "reconcile", "download"] as const;

export interface ProviderAccountFormProps {
  initial?: Partial<PaymentProviderAccountView>;
  mode: "create" | "update";
  partnerAccountOptions?: readonly PaymentProviderAccountView[];
  onCancel(): void;
  onSubmit(
    draft: PaymentProviderAccountDraft | PaymentProviderAccountUpdateDraft,
  ): Promise<void> | void;
}

interface FormState {
  accountNo: string;
  providerCode: PaymentProviderCode;
  merchantId: string;
  accountMode: PaymentProviderAccountMode;
  partnerProviderAccountId: string;
  environment: PaymentProviderEnvironment;
  countryCode: string;
  settlementCurrency: string;
  secretRef: string;
  webhookSecretRef: string;
  certificateRef: string;
  status: PaymentProviderAccountStatus;
  metadataAppId: string;
  metadataMerchantSerialNo: string;
  metadataSignType: string;
  metadataReturnUrl: string;
  capabilities: Record<string, boolean>;
}

function deriveInitialState(
  initial: Partial<PaymentProviderAccountView> | undefined,
): FormState {
  const metadata = initial?.metadata ?? {};
  return {
    accountNo: initial?.accountNo ?? "",
    providerCode: initial?.providerCode ?? "stripe",
    merchantId: initial?.merchantId ?? "",
    accountMode: initial?.accountMode ?? "direct",
    partnerProviderAccountId: initial?.partnerProviderAccountId ?? "",
    environment: initial?.environment ?? "sandbox",
    countryCode: initial?.countryCode ?? "CN",
    settlementCurrency: initial?.settlementCurrency ?? "CNY",
    secretRef: initial?.secretRef ?? "",
    webhookSecretRef: initial?.webhookSecretRef ?? "",
    certificateRef: initial?.certificateRef ?? "",
    status: initial?.status ?? "active",
    metadataAppId: typeof metadata.appId === "string" ? metadata.appId : "",
    metadataMerchantSerialNo: typeof metadata.merchantSerialNo === "string" ? metadata.merchantSerialNo : "",
    metadataSignType: typeof metadata.signType === "string" ? metadata.signType : "RSA2",
    metadataReturnUrl: typeof metadata.returnUrl === "string" ? metadata.returnUrl : "",
    capabilities: {
      pay: initial?.capabilities?.pay ?? true,
      refund: initial?.capabilities?.refund ?? true,
      close: initial?.capabilities?.close ?? true,
      query: initial?.capabilities?.query ?? true,
      reconcile: initial?.capabilities?.reconcile ?? false,
      download: initial?.capabilities?.download ?? false,
    },
  };
}

export function ProviderAccountForm(props: ProviderAccountFormProps) {
  const [state, setState] = React.useState<FormState>(() =>
    deriveInitialState(props.initial),
  );
  const [submitting, setSubmitting] = React.useState(false);
  const [error, setError] = React.useState<string | undefined>();

  const isCreate = props.mode === "create";

  function update<K extends keyof FormState>(key: K, value: FormState[K]) {
    setState((prev) => ({ ...prev, [key]: value }));
  }

  function buildMetadata(): Record<string, unknown> {
    const metadata: Record<string, unknown> = {};
    if (state.metadataAppId) {
      metadata.appId = state.metadataAppId;
    }
    if (state.metadataMerchantSerialNo) {
      metadata.merchantSerialNo = state.metadataMerchantSerialNo;
    }
    if (state.metadataSignType) {
      metadata.signType = state.metadataSignType;
    }
    if (state.metadataReturnUrl) {
      metadata.returnUrl = state.metadataReturnUrl;
    }
    return metadata;
  }

  function buildCapabilities() {
    const capabilities: Record<string, boolean> = {};
    for (const key of CAPABILITY_KEYS) {
      capabilities[key] = state.capabilities[key] ?? false;
    }
    return capabilities;
  }

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    if (!state.accountNo.trim() || !state.merchantId.trim() || !state.secretRef.trim()) {
      setError("Account no, merchant id, and secret reference are required.");
      return;
    }
    if (state.accountMode === "partner" && !state.partnerProviderAccountId.trim() && isCreate) {
      setError("Partner provider account is required when account mode is partner.");
      return;
    }
    setSubmitting(true);
    try {
      const metadata = buildMetadata();
      const capabilities = buildCapabilities();
      if (isCreate) {
        const draft: PaymentProviderAccountDraft = {
          accountNo: state.accountNo.trim(),
          providerCode: state.providerCode,
          merchantId: state.merchantId.trim(),
          accountMode: state.accountMode,
          ...(state.partnerProviderAccountId.trim()
            ? { partnerProviderAccountId: state.partnerProviderAccountId.trim() }
            : {}),
          environment: state.environment,
          countryCode: state.countryCode.trim().toUpperCase() || "CN",
          settlementCurrency: state.settlementCurrency.trim().toUpperCase() || "CNY",
          secretRef: state.secretRef.trim(),
          ...(state.webhookSecretRef.trim() ? { webhookSecretRef: state.webhookSecretRef.trim() } : {}),
          ...(state.certificateRef.trim() ? { certificateRef: state.certificateRef.trim() } : {}),
          capabilities,
          status: state.status,
          metadata,
        };
        await props.onSubmit(draft);
      } else {
        const draft: PaymentProviderAccountUpdateDraft = {
          merchantId: state.merchantId.trim(),
          accountMode: state.accountMode,
          ...(state.partnerProviderAccountId.trim()
            ? { partnerProviderAccountId: state.partnerProviderAccountId.trim() }
            : {}),
          environment: state.environment,
          countryCode: state.countryCode.trim().toUpperCase() || "CN",
          settlementCurrency: state.settlementCurrency.trim().toUpperCase() || "CNY",
          ...(state.secretRef.trim() ? { secretRef: state.secretRef.trim() } : {}),
          ...(state.webhookSecretRef.trim() ? { webhookSecretRef: state.webhookSecretRef.trim() } : {}),
          ...(state.certificateRef.trim() ? { certificateRef: state.certificateRef.trim() } : {}),
          capabilities,
          status: state.status,
          metadata,
        };
        await props.onSubmit(draft);
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit provider account form.");
    } finally {
      setSubmitting(false);
    }
  }

  const showAlipayFields = state.providerCode === "alipay";
  const showWeChatFields = state.providerCode === "wechat_pay";
  const showStripeFields = state.providerCode === "stripe";
  const showSandboxFields = state.providerCode === "sandbox";
  const showPartnerFields = state.accountMode === "partner";

  return (
    <form
      className="space-y-4"
      onSubmit={handleSubmit}
      aria-label="Provider account form"
    >
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <AdminFieldLabel label="Account No" htmlFor="provider-account-no" required>
          <Input
            id="provider-account-no"
            value={state.accountNo}
            onChange={(event) => update("accountNo", event.target.value)}
            disabled={!isCreate}
            placeholder="e.g., stripe-live-primary"
            required
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Provider" htmlFor="provider-code" required>
          <Select
            value={state.providerCode}
            onValueChange={(value) => update("providerCode", value as PaymentProviderCode)}
            disabled={!isCreate}
          >
            <SelectTrigger id="provider-code">
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
        <AdminFieldLabel label="Merchant ID" htmlFor="provider-merchant-id" required>
          <Input
            id="provider-merchant-id"
            value={state.merchantId}
            onChange={(event) => update("merchantId", event.target.value)}
            placeholder="e.g., merchant_001 or acct_xxx"
            required
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Environment" htmlFor="provider-environment" required>
          <Select
            value={state.environment}
            onValueChange={(value) =>
              update("environment", value as PaymentProviderEnvironment)
            }
          >
            <SelectTrigger id="provider-environment">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ENVIRONMENT_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Account Mode" htmlFor="provider-account-mode" required>
          <Select
            value={state.accountMode}
            onValueChange={(value) =>
              update("accountMode", value as PaymentProviderAccountMode)
            }
          >
            <SelectTrigger id="provider-account-mode">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {ACCOUNT_MODE_OPTIONS.map((option) => (
                <SelectItem key={option.value} value={option.value}>
                  {option.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </AdminFieldLabel>
        <AdminFieldLabel label="Status" htmlFor="provider-status">
          <Select
            value={state.status}
            onValueChange={(value) =>
              update("status", value as PaymentProviderAccountStatus)
            }
          >
            <SelectTrigger id="provider-status">
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
        <AdminFieldLabel label="Country Code" htmlFor="provider-country-code">
          <Input
            id="provider-country-code"
            value={state.countryCode}
            onChange={(event) => update("countryCode", event.target.value)}
            maxLength={2}
            placeholder="CN"
          />
        </AdminFieldLabel>
        <AdminFieldLabel label="Settlement Currency" htmlFor="provider-settlement-currency">
          <Input
            id="provider-settlement-currency"
            value={state.settlementCurrency}
            onChange={(event) => update("settlementCurrency", event.target.value)}
            maxLength={3}
            placeholder="CNY"
          />
        </AdminFieldLabel>
      </div>

      {showPartnerFields ? (
        <div className="rounded-md border border-[var(--sdk-color-border-subtle)] bg-[var(--sdk-color-bg-subtle)] p-4">
          <div className="mb-2 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
            Partner / ISV Configuration
          </div>
          <AdminFieldLabel
            label="Partner Provider Account"
            htmlFor="provider-partner-account-id"
            required={isCreate}
          >
            <Select
              value={state.partnerProviderAccountId}
              onValueChange={(value) => update("partnerProviderAccountId", value)}
              disabled={!isCreate && Boolean(props.initial?.partnerProviderAccountId)}
            >
              <SelectTrigger id="provider-partner-account-id">
                <SelectValue placeholder="Select partner account..." />
              </SelectTrigger>
              <SelectContent>
                {(props.partnerAccountOptions ?? []).map((account) => (
                  <SelectItem key={account.id} value={account.id}>
                    {account.accountNo} ({account.providerCode})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </AdminFieldLabel>
          <p className="mt-2 text-xs text-[var(--sdk-color-text-secondary)]">
            Sub-merchants (Alipay sub_appid / WeChat sub_mch_id / Stripe Connected Account)
            are managed under the partner account in the Sub-Merchants tab.
          </p>
        </div>
      ) : null}

      <div className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
        <div className="mb-3 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Credential References (env var names, never plaintext)
        </div>
        <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
          <AdminFieldLabel
            label={secretRefLabel(state.providerCode)}
            htmlFor="provider-secret-ref"
            required
          >
            <Input
              id="provider-secret-ref"
              value={state.secretRef}
              onChange={(event) => update("secretRef", event.target.value)}
              placeholder={secretRefPlaceholder(state.providerCode)}
              required
            />
          </AdminFieldLabel>
          {showStripeFields || showWeChatFields ? (
            <AdminFieldLabel
              label={webhookSecretRefLabel(state.providerCode)}
              htmlFor="provider-webhook-secret-ref"
            >
              <Input
                id="provider-webhook-secret-ref"
                value={state.webhookSecretRef}
                onChange={(event) => update("webhookSecretRef", event.target.value)}
                placeholder={webhookSecretRefPlaceholder(state.providerCode)}
              />
            </AdminFieldLabel>
          ) : null}
          {showAlipayFields || showWeChatFields ? (
            <AdminFieldLabel
              label={certificateRefLabel(state.providerCode)}
              htmlFor="provider-certificate-ref"
            >
              <Input
                id="provider-certificate-ref"
                value={state.certificateRef}
                onChange={(event) => update("certificateRef", event.target.value)}
                placeholder={certificateRefPlaceholder(state.providerCode)}
              />
            </AdminFieldLabel>
          ) : null}
        </div>
        <p className="mt-3 text-xs text-[var(--sdk-color-text-secondary)]">
          These fields store environment variable names (e.g., STRIPE_SECRET_KEY,
          ALIPAY_MERCHANT_PRIVATE_KEY). The runtime resolves them via the secret store;
          plaintext secrets are never persisted in the database.
        </p>
      </div>

      {showAlipayFields || showWeChatFields ? (
        <div className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
          <div className="mb-3 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
            Provider Metadata
          </div>
          <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
            <AdminFieldLabel label="App ID" htmlFor="provider-metadata-app-id">
              <Input
                id="provider-metadata-app-id"
                value={state.metadataAppId}
                onChange={(event) => update("metadataAppId", event.target.value)}
                placeholder={showAlipayFields ? "Alipay open platform app id" : "WeChat mini program app id"}
              />
            </AdminFieldLabel>
            {showAlipayFields ? (
              <AdminFieldLabel label="Sign Type" htmlFor="provider-metadata-sign-type">
                <Select
                  value={state.metadataSignType}
                  onValueChange={(value) => update("metadataSignType", value)}
                >
                  <SelectTrigger id="provider-metadata-sign-type">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="RSA2">RSA2 (recommended)</SelectItem>
                    <SelectItem value="RSA">RSA</SelectItem>
                  </SelectContent>
                </Select>
              </AdminFieldLabel>
            ) : null}
            {showWeChatFields ? (
              <AdminFieldLabel
                label="Merchant Serial No"
                htmlFor="provider-metadata-merchant-serial-no"
              >
                <Input
                  id="provider-metadata-merchant-serial-no"
                  value={state.metadataMerchantSerialNo}
                  onChange={(event) => update("metadataMerchantSerialNo", event.target.value)}
                  placeholder="WeChat API v3 merchant certificate serial number"
                />
              </AdminFieldLabel>
            ) : null}
            <AdminFieldLabel label="Return URL" htmlFor="provider-metadata-return-url">
              <Input
                id="provider-metadata-return-url"
                value={state.metadataReturnUrl}
                onChange={(event) => update("metadataReturnUrl", event.target.value)}
                placeholder="Optional override return URL"
              />
            </AdminFieldLabel>
          </div>
        </div>
      ) : null}

      {showSandboxFields ? (
        <p className="text-xs text-[var(--sdk-color-text-secondary)]">
          Sandbox provider only requires the secret reference. Use it for local
          development and integration tests; no metadata is required.
        </p>
      ) : null}

      <div className="rounded-md border border-[var(--sdk-color-border-subtle)] p-4">
        <div className="mb-3 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
          Capabilities
        </div>
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          {CAPABILITY_KEYS.map((key) => (
            <label
              key={key}
              className="flex items-center gap-2 text-sm"
              htmlFor={`provider-capability-${key}`}
            >
              <Switch
                id={`provider-capability-${key}`}
                checked={state.capabilities[key] ?? false}
                onCheckedChange={(checked) =>
                  setState((prev) => ({
                    ...prev,
                    capabilities: { ...prev.capabilities, [key]: checked },
                  }))
                }
              />
              <span className="capitalize">{key}</span>
            </label>
          ))}
        </div>
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
        <Button type="button" variant="ghost" onClick={props.onCancel} disabled={submitting} title="Saving in progress...">
          Cancel
        </Button>
        <Button type="submit" disabled={submitting} title="Saving in progress...">
          {submitting ? "Saving..." : isCreate ? "Create Account" : "Update Account"}
        </Button>
      </div>
    </form>
  );
}

function secretRefLabel(providerCode: PaymentProviderCode): string {
  if (providerCode === "stripe") return "Stripe Secret Key Env Var";
  if (providerCode === "alipay") return "Alipay Merchant Private Key Env Var";
  if (providerCode === "wechat_pay") return "WeChat API v3 Key Env Var";
  return "Secret Env Var";
}

function secretRefPlaceholder(providerCode: PaymentProviderCode): string {
  if (providerCode === "stripe") return "STRIPE_SECRET_KEY";
  if (providerCode === "alipay") return "ALIPAY_MERCHANT_PRIVATE_KEY";
  if (providerCode === "wechat_pay") return "WECHAT_PAY_API_V3_KEY";
  return "SANDBOX_SECRET";
}

function webhookSecretRefLabel(providerCode: PaymentProviderCode): string {
  if (providerCode === "stripe") return "Stripe Webhook Signing Secret Env Var";
  if (providerCode === "wechat_pay") return "WeChat API v3 Key (Webhook) Env Var";
  return "Webhook Secret Env Var";
}

function webhookSecretRefPlaceholder(providerCode: PaymentProviderCode): string {
  if (providerCode === "stripe") return "STRIPE_WEBHOOK_SECRET";
  if (providerCode === "wechat_pay") return "WECHAT_PAY_API_V3_KEY";
  return "WEBHOOK_SECRET";
}

function certificateRefLabel(providerCode: PaymentProviderCode): string {
  if (providerCode === "alipay") return "Alipay Public Key Env Var";
  if (providerCode === "wechat_pay") return "WeChat Platform Certificate Env Var";
  return "Certificate Env Var";
}

function certificateRefPlaceholder(providerCode: PaymentProviderCode): string {
  if (providerCode === "alipay") return "ALIPAY_PUBLIC_KEY";
  if (providerCode === "wechat_pay") return "WECHAT_PAY_PLATFORM_CERT";
  return "CERTIFICATE_PEM";
}
