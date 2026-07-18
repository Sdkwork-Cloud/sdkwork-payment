/**
 * Provider admin workspace.
 *
 * Two-tab workspace:
 *   1. Provider Accounts — list + create/edit form + test/rotate actions
 *   2. Sub-Merchants — manage sub-merchants under a selected partner account
 *
 * Uses an external store subscription pattern (subscribe/getState) so the host
 * app can wire it into React's useSyncExternalStore if needed.
 */

import * as React from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  Input,
  SettingsSection,
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@sdkwork/ui-pc-react";
import { AdminFieldLabel } from "@sdkwork/payment-pc-admin-core";
import { ProviderAccountForm } from "../components/ProviderAccountForm";
import { ProviderAccountList } from "../components/ProviderAccountList";
import { SubMerchantManager } from "../components/SubMerchantManager";
import type {
  PaymentCredentialRotateDraft,
  PaymentProviderAccountDraft,
  PaymentProviderAccountTestOptions,
  PaymentProviderAccountUpdateDraft,
  PaymentProviderAdminController,
  PaymentProviderAdminState,
  PaymentProviderAccountView,
  PaymentSubMerchantDraft,
  PaymentSubMerchantUpdateDraft,
} from "../types/provider-admin-types";

export interface PaymentProviderAdminWorkspaceProps {
  controller: PaymentProviderAdminController;
  capabilities: PaymentProviderAdminCapabilities;
  title?: string;
  description?: string;
}

export interface PaymentProviderAdminCapabilities {
  canCreateProviderAccount: boolean;
  canUpdateProviderAccount: boolean;
  canTestProviderAccount: boolean;
  canRotateProviderCredentials: boolean;
  canCreateSubMerchant: boolean;
  canUpdateSubMerchant: boolean;
  canDeleteSubMerchant: boolean;
}

type DialogState =
  | { kind: "closed" }
  | { kind: "create" }
  | { kind: "edit"; account: PaymentProviderAccountView }
  | { kind: "test"; account: PaymentProviderAccountView }
  | { kind: "rotate"; account: PaymentProviderAccountView };

export function PaymentProviderAdminWorkspace(
  props: PaymentProviderAdminWorkspaceProps,
) {
  const { controller } = props;
  const [state, setState] = React.useState<PaymentProviderAdminState>(() => controller.getState());
  const [tab, setTab] = React.useState<"accounts" | "submerchants">("accounts");
  const [dialog, setDialog] = React.useState<DialogState>({ kind: "closed" });

  React.useEffect(() => {
    return controller.subscribe(() => {
      setState(controller.getState());
    });
  }, [controller]);

  React.useEffect(() => {
    void controller.load().then(setState).catch(() => {
      // error already surfaced via controller state.lastError
    });
  }, [controller]);

  const partnerAccounts = React.useMemo(
    () => state.providerAccounts.filter((account) => account.accountMode === "partner"),
    [state.providerAccounts],
  );

  const selectedPartnerAccount = state.selectedProviderAccount
    ? state.selectedProviderAccount
    : partnerAccounts[0];

  const visibleSubMerchants = React.useMemo(() => {
    if (selectedPartnerAccount) {
      return state.subMerchants.filter(
        (merchant) => merchant.providerAccountId === selectedPartnerAccount.id,
      );
    }
    return state.subMerchants;
  }, [state.subMerchants, selectedPartnerAccount]);

  async function handleCreate(draft: PaymentProviderAccountDraft) {
    await controller.createProviderAccount(draft);
    setDialog({ kind: "closed" });
  }

  async function handleUpdate(draft: PaymentProviderAccountUpdateDraft) {
    if (dialog.kind !== "edit") {
      return;
    }
    await controller.updateProviderAccount(dialog.account.id, draft);
    setDialog({ kind: "closed" });
  }

  async function handleTest() {
    if (dialog.kind !== "test") {
      return;
    }
    const options: PaymentProviderAccountTestOptions = {
      environment: dialog.account.environment,
      dryRun: false,
    };
    await controller.testProviderAccount(dialog.account.id, options);
  }

  async function handleRotate(draft: PaymentCredentialRotateDraft) {
    if (dialog.kind !== "rotate") {
      return;
    }
    await controller.rotateProviderAccountCredentials(dialog.account.id, draft);
    setDialog({ kind: "closed" });
  }

  function handleSelect(account: PaymentProviderAccountView) {
    controller.selectProviderAccount(account.id);
    if (account.accountMode === "partner") {
      void controller.loadMoreSubMerchants(account.id);
      setTab("submerchants");
    }
  }

  return (
    <section className="space-y-6" data-slot="payment-provider-admin-workspace">
      <header className="space-y-2">
        <h2 className="text-lg font-semibold text-[var(--sdk-color-text)]">
          {props.title ?? "Payment provider administration"}
        </h2>
        <p className="text-sm text-[var(--sdk-color-text-secondary)]">
          {props.description ??
            "Configure Stripe, Alipay, WeChat Pay, and sandbox provider accounts. Supports direct (merchant self-connection) and partner/ISV (sub-merchant) modes."}
        </p>
      </header>

      {state.lastError ? (
        <div
          role="alert"
          className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
        >
          {state.lastError}
        </div>
      ) : null}

      {state.lastTestResult ? (
        <div
          role="status"
          className={
            "rounded-md border p-3 text-sm " +
            (state.lastTestResult.ok
              ? "border-[var(--sdk-color-border-success)] bg-[var(--sdk-color-bg-success-subtle)] text-[var(--sdk-color-text-success)]"
              : "border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] text-[var(--sdk-color-text-error)]")
          }
        >
          <div className="font-medium">
            {state.lastTestResult.ok ? "Credentials verified" : "Credential test failed"}
          </div>
          <div className="mt-1 text-xs">
            Provider: {state.lastTestResult.providerCode} · Environment:{" "}
            {state.lastTestResult.environment}
            {typeof state.lastTestResult.pspResponseTimeMs === "number"
              ? ` · Latency: ${state.lastTestResult.pspResponseTimeMs}ms`
              : ""}
            {state.lastTestResult.diagnostic ? ` · ${state.lastTestResult.diagnostic}` : ""}
          </div>
        </div>
      ) : null}

      <Tabs
        value={tab}
        onValueChange={(value) => setTab(value as "accounts" | "submerchants")}
      >
        <TabsList>
          <TabsTrigger value="accounts">Provider Accounts</TabsTrigger>
          <TabsTrigger value="submerchants">Sub-Merchants</TabsTrigger>
        </TabsList>
        <TabsContent value="accounts">
          <SettingsSection
            title="Provider Accounts"
            description="Manage PSP credentials, environment, and capabilities per provider account. Secrets are referenced by env var name only."
            actions={
              props.capabilities.canCreateProviderAccount ? (
                <Button
                  type="button"
                  size="sm"
                  onClick={() => setDialog({ kind: "create" })}
                  disabled={state.status === "saving" || state.status === "loading"}
                >
                  Create provider account
                </Button>
              ) : null
            }
          >
            <ProviderAccountList
              accounts={state.providerAccounts}
              pageInfo={state.listPageInfo?.providerAccounts}
              selectedId={state.selectedProviderAccount?.id}
              busy={state.status === "saving" || state.status === "loading"}
              canCreate={props.capabilities.canCreateProviderAccount}
              canEdit={props.capabilities.canUpdateProviderAccount}
              canRotate={props.capabilities.canRotateProviderCredentials}
              canTest={props.capabilities.canTestProviderAccount}
              onSelect={handleSelect}
              onEdit={(account) => setDialog({ kind: "edit", account })}
              onTest={(account) => setDialog({ kind: "test", account })}
              onRotate={(account) => setDialog({ kind: "rotate", account })}
              onLoadMore={() => void controller.loadMoreProviderAccounts()}
              onCreate={() => setDialog({ kind: "create" })}
            />
          </SettingsSection>
        </TabsContent>
        <TabsContent value="submerchants">
          <SettingsSection
            title="Sub-Merchants"
            description="Sub-merchant records under a partner/ISV provider account. Each maps to Alipay sub_appid, WeChat sub_mch_id, or Stripe Connected Account."
          >
            {partnerAccounts.length > 0 ? (
              <div className="space-y-2">
                <label className="text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
                  Selected partner account
                </label>
                <select
                  className="w-full rounded-md border border-[var(--sdk-color-border-subtle)] bg-[var(--sdk-color-bg)] px-3 py-2 text-sm"
                  value={selectedPartnerAccount?.id ?? ""}
                  onChange={(event) => {
                    const nextId = event.target.value;
                    if (!nextId) {
                      controller.selectProviderAccount(undefined);
                      return;
                    }
                    const account = partnerAccounts.find((item) => item.id === nextId);
                    if (account) {
                      controller.selectProviderAccount(account.id);
                      void controller.loadMoreSubMerchants(account.id);
                    }
                  }}
                >
                  {partnerAccounts.map((account) => (
                    <option key={account.id} value={account.id}>
                      {account.accountNo} ({account.providerCode})
                    </option>
                  ))}
                </select>
              </div>
            ) : null}
            <SubMerchantManager
              partnerAccount={selectedPartnerAccount}
              subMerchants={visibleSubMerchants}
              pageInfo={state.listPageInfo?.subMerchants}
              busy={state.status === "saving" || state.status === "loading"}
              canCreate={props.capabilities.canCreateSubMerchant}
              canDelete={props.capabilities.canDeleteSubMerchant}
              canUpdate={props.capabilities.canUpdateSubMerchant}
              onCreate={(draft) => void controller.createSubMerchant(draft)}
              onUpdate={(id, draft) => void controller.updateSubMerchant(id, draft)}
              onDelete={(id) => void controller.deleteSubMerchant(id)}
              onLoadMore={() =>
                void controller.loadMoreSubMerchants(selectedPartnerAccount?.id)
              }
            />
          </SettingsSection>
        </TabsContent>
      </Tabs>

      <Dialog
        open={dialog.kind === "create" || dialog.kind === "edit"}
        onOpenChange={(open) => {
          if (!open) {
            setDialog({ kind: "closed" });
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {dialog.kind === "create" ? "Create provider account" : "Edit provider account"}
            </DialogTitle>
          </DialogHeader>
          {dialog.kind === "create" || dialog.kind === "edit" ? (
            <ProviderAccountForm
              mode={dialog.kind === "create" ? "create" : "update"}
              initial={dialog.kind === "edit" ? dialog.account : undefined}
              partnerAccountOptions={partnerAccounts}
              onCancel={() => setDialog({ kind: "closed" })}
              onSubmit={
                dialog.kind === "create"
                  ? (draft) => handleCreate(draft as PaymentProviderAccountDraft)
                  : (draft) => handleUpdate(draft as PaymentProviderAccountUpdateDraft)
              }
            />
          ) : null}
        </DialogContent>
      </Dialog>

      <Dialog
        open={dialog.kind === "test"}
        onOpenChange={(open) => {
          if (!open) {
            setDialog({ kind: "closed" });
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Test provider credentials</DialogTitle>
          </DialogHeader>
          {dialog.kind === "test" ? (
            <div className="space-y-3">
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">
                This will invoke the lowest-cost PSP API to verify connectivity for{" "}
                <strong>{dialog.account.accountNo}</strong> ({dialog.account.providerCode} /{" "}
                {dialog.account.environment}). The result updates the provider account's
                <code className="mx-1 rounded bg-[var(--sdk-color-bg-subtle)] px-1 text-xs">
                  last_tested_at
                </code>
                and
                <code className="mx-1 rounded bg-[var(--sdk-color-bg-subtle)] px-1 text-xs">
                  last_test_status
                </code>
                fields.
              </p>
              <div className="flex justify-end gap-2">
                <Button
                  type="button"
                  variant="ghost"
                  onClick={() => setDialog({ kind: "closed" })}
                >
                  Cancel
                </Button>
                <Button
                  type="button"
                  onClick={() => void handleTest().then(() => setDialog({ kind: "closed" }))}
                  disabled={state.status === "testing"}
                >
                  {state.status === "testing" ? "Testing..." : "Run test"}
                </Button>
              </div>
            </div>
          ) : null}
        </DialogContent>
      </Dialog>

      <Dialog
        open={dialog.kind === "rotate"}
        onOpenChange={(open) => {
          if (!open) {
            setDialog({ kind: "closed" });
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Rotate provider credentials</DialogTitle>
          </DialogHeader>
          {dialog.kind === "rotate" ? (
            <RotateCredentialsDialog
              account={dialog.account}
              busy={state.status === "saving"}
              onCancel={() => setDialog({ kind: "closed" })}
              onSubmit={handleRotate}
            />
          ) : null}
        </DialogContent>
      </Dialog>
    </section>
  );
}

// Credential rotation form: replaces the legacy window.prompt anti-pattern with a structured
// Dialog + Input/Switch form, supporting simultaneous rotation of secretRef / webhookSecretRef /
// certificateRef with control over whether to invalidate previous credentials.
interface RotateCredentialsDialogProps {
  account: PaymentProviderAccountView;
  busy: boolean;
  onCancel(): void;
  onSubmit(draft: PaymentCredentialRotateDraft): Promise<void> | void;
}

interface RotateFormState {
  secretRef: string;
  webhookSecretRef: string;
  certificateRef: string;
  invalidatePrevious: boolean;
}

function RotateCredentialsDialog(props: RotateCredentialsDialogProps) {
  // Pre-fill with the current account's credential references on open, so they can be edited in place
  const [state, setState] = React.useState<RotateFormState>(() => ({
    secretRef: props.account.secretRef,
    webhookSecretRef: props.account.webhookSecretRef ?? "",
    certificateRef: props.account.certificateRef ?? "",
    invalidatePrevious: true,
  }));
  const [submitting, setSubmitting] = React.useState(false);
  const [error, setError] = React.useState<string | undefined>();

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(undefined);
    setSubmitting(true);
    try {
      const draft: PaymentCredentialRotateDraft = {
        secretRef: state.secretRef.trim(),
        ...(state.webhookSecretRef.trim()
          ? { webhookSecretRef: state.webhookSecretRef.trim() }
          : {}),
        ...(state.certificateRef.trim()
          ? { certificateRef: state.certificateRef.trim() }
          : {}),
        invalidatePrevious: state.invalidatePrevious,
      };
      await props.onSubmit(draft);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to rotate credentials.");
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <form
      className="space-y-4"
      onSubmit={handleSubmit}
      aria-label="Rotate credentials form"
    >
      <p className="text-sm text-[var(--sdk-color-text-secondary)]">
        Provide new secret env var names below. The previous env var will be marked
        as deprecated in account metadata; the underlying secret is not revoked
        automatically — rotate it in your secret store separately.
      </p>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <AdminFieldLabel label="Secret Env Var" htmlFor="rotate-secret-ref">
          <Input
            id="rotate-secret-ref"
            value={state.secretRef}
            onChange={(event) =>
              setState((prev) => ({ ...prev, secretRef: event.target.value }))
            }
            placeholder="New secret env var name"
          />
        </AdminFieldLabel>
        <AdminFieldLabel
          label="Webhook Secret Env Var"
          htmlFor="rotate-webhook-secret-ref"
        >
          <Input
            id="rotate-webhook-secret-ref"
            value={state.webhookSecretRef}
            onChange={(event) =>
              setState((prev) => ({ ...prev, webhookSecretRef: event.target.value }))
            }
            placeholder="New webhook secret env var name"
          />
        </AdminFieldLabel>
        <AdminFieldLabel
          label="Certificate Env Var"
          htmlFor="rotate-certificate-ref"
        >
          <Input
            id="rotate-certificate-ref"
            value={state.certificateRef}
            onChange={(event) =>
              setState((prev) => ({ ...prev, certificateRef: event.target.value }))
            }
            placeholder="New certificate env var name"
          />
        </AdminFieldLabel>
        <AdminFieldLabel
          label="Invalidate previous credentials"
          htmlFor="rotate-invalidate-previous"
        >
          <Switch
            id="rotate-invalidate-previous"
            checked={state.invalidatePrevious}
            onCheckedChange={(checked) =>
              setState((prev) => ({ ...prev, invalidatePrevious: checked }))
            }
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
        <Button
          type="button"
          variant="ghost"
          onClick={props.onCancel}
          disabled={submitting || props.busy}
        >
          Cancel
        </Button>
        <Button type="submit" disabled={submitting || props.busy}>
          {submitting || props.busy ? "Rotating..." : "Rotate credentials"}
        </Button>
      </div>
    </form>
  );
}

// Re-export commonly used Tabs sub-components for host apps that want to wrap them.
export { Tabs as PaymentProviderAdminTabs, TabsList, TabsTrigger, TabsContent };
