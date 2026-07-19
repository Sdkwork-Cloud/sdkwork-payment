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
  Switch,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@sdkwork/ui-pc-react";
import {
  AdminFieldLabel,
  PaymentAdminI18nBoundary,
  PaymentAdminTabsContent,
  PaymentAdminTabsList,
  PaymentAdminTabsTrigger,
  PaymentAdminWorkspace,
} from "@sdkwork/payment-pc-admin-core";
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
  section?: PaymentProviderAdminSection;
  title?: string;
  description?: string;
}

export type PaymentProviderAdminSection = "accounts" | "submerchants";

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
  const [tab, setTab] = React.useState<PaymentProviderAdminSection>("accounts");
  const activeSection = props.section ?? tab;
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
    if (draft.status === "active") {
      await controller.updateProviderAccount(dialog.account.id, {
        ...draft,
        status: "inactive",
      });
      const result = await controller.testProviderAccount(dialog.account.id, {
        environment: draft.environment ?? dialog.account.environment,
        dryRun: true,
      });
      if (!result.ok) {
        throw new Error(result.diagnostic ?? "Provider account readiness validation failed.");
      }
      await controller.updateProviderAccount(dialog.account.id, { status: "active" });
    } else {
      await controller.updateProviderAccount(dialog.account.id, draft);
    }
    setDialog({ kind: "closed" });
  }

  async function handleTest() {
    if (dialog.kind !== "test") {
      return;
    }
    const options: PaymentProviderAccountTestOptions = {
      environment: dialog.account.environment,
      dryRun: true,
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
      if (!props.section) {
        setTab("submerchants");
      }
    }
  }

  return (
    <PaymentAdminI18nBoundary>
      <PaymentAdminWorkspace
        data-slot="payment-provider-admin-workspace"
        description={props.description}
        error={state.lastError}
        title={props.title ?? "Provider accounts & sub-merchants"}
      >
        {state.lastTestResult ? (
          <div
            role="status"
            className={
              "border-l-2 px-3 py-2 text-sm " +
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
          value={activeSection}
          onValueChange={(value) => {
            if (!props.section) {
              setTab(value as PaymentProviderAdminSection);
            }
          }}
        >
          {!props.section ? (
            <PaymentAdminTabsList aria-label="Payment provider sections">
              <PaymentAdminTabsTrigger value="accounts">Provider accounts</PaymentAdminTabsTrigger>
              <PaymentAdminTabsTrigger value="submerchants">Sub-merchants</PaymentAdminTabsTrigger>
            </PaymentAdminTabsList>
          ) : null}
          <PaymentAdminTabsContent value="accounts">
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
          </PaymentAdminTabsContent>
          <PaymentAdminTabsContent value="submerchants">
            <div className="space-y-4">
              {partnerAccounts.length > 0 ? (
                <div className="flex flex-col gap-2 sm:max-w-sm">
                  <label
                    className="text-xs font-medium text-[var(--sdk-color-text-secondary)]"
                    htmlFor="payment-provider-partner-account"
                  >
                    Selected partner account
                  </label>
                  <select
                    className="h-9 w-full rounded-[var(--sdk-radius-control)] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel)] px-3 text-sm text-[var(--sdk-color-text-primary)] outline-none focus:border-[var(--sdk-color-border-focus)] focus:ring-2 focus:ring-[var(--sdk-color-border-focus)]"
                    id="payment-provider-partner-account"
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
            </div>
          </PaymentAdminTabsContent>
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
                Validate the saved credential references and provider adapter for{" "}
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
      </PaymentAdminWorkspace>
    </PaymentAdminI18nBoundary>
  );
}

// Credential rotation form: replaces the legacy window.prompt anti-pattern with a structured
// Dialog + write-only credential fields. Existing values are never loaded into browser state.
interface RotateCredentialsDialogProps {
  account: PaymentProviderAccountView;
  busy: boolean;
  onCancel(): void;
  onSubmit(draft: PaymentCredentialRotateDraft): Promise<void> | void;
}

interface RotateFormState {
  primarySecret: string;
  webhookSecret: string;
  certificate: string;
  invalidatePrevious: boolean;
}

function RotateCredentialsDialog(props: RotateCredentialsDialogProps) {
  const [state, setState] = React.useState<RotateFormState>(() => ({
    primarySecret: "",
    webhookSecret: "",
    certificate: "",
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
        primarySecret: state.primarySecret.trim(),
        ...(state.webhookSecret.trim()
          ? { webhookSecret: state.webhookSecret.trim() }
          : {}),
        ...(state.certificate.trim()
          ? { certificate: state.certificate.trim() }
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
        New credential versions are encrypted in the database. Previous active versions
        are superseded after this operation succeeds.
      </p>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <AdminFieldLabel label="Primary Credential" htmlFor="rotate-primary-secret" required>
          <textarea
            id="rotate-primary-secret"
            value={state.primarySecret}
            onChange={(event) =>
              setState((prev) => ({ ...prev, primarySecret: event.target.value }))
            }
            placeholder="Enter new credential value"
            required
            rows={5}
            autoComplete="new-password"
            className="w-full resize-y rounded-md border border-[var(--sdk-color-border)] bg-[var(--sdk-color-bg-surface)] px-3 py-2 font-mono text-sm text-[var(--sdk-color-text-primary)]"
          />
        </AdminFieldLabel>
        <AdminFieldLabel
          label="Webhook / API v3 Secret"
          htmlFor="rotate-webhook-secret"
        >
          <Input
            id="rotate-webhook-secret"
            type="password"
            value={state.webhookSecret}
            onChange={(event) =>
              setState((prev) => ({ ...prev, webhookSecret: event.target.value }))
            }
            placeholder="Enter new secret value"
            autoComplete="new-password"
          />
        </AdminFieldLabel>
        <AdminFieldLabel
          label="Certificate / Provider Public Key"
          htmlFor="rotate-certificate"
        >
          <textarea
            id="rotate-certificate"
            value={state.certificate}
            onChange={(event) =>
              setState((prev) => ({ ...prev, certificate: event.target.value }))
            }
            placeholder="Enter new PEM value"
            rows={5}
            autoComplete="new-password"
            className="w-full resize-y rounded-md border border-[var(--sdk-color-border)] bg-[var(--sdk-color-bg-surface)] px-3 py-2 font-mono text-sm text-[var(--sdk-color-text-primary)]"
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
