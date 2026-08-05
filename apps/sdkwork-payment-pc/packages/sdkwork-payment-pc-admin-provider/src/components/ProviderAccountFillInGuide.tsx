/**
 * Fill-in guide for the provider account create/edit dialog.
 *
 * A nested dialog reachable from a link button in the dialog header. Explains
 * each account field and how to obtain the credential material (PEM private
 * keys, public keys, certificates) for Alipay, WeChat Pay, and Stripe — the
 * same guidance a PSP console would surface next to its connection form.
 */

import * as React from "react";
import { CircleHelp } from "lucide-react";
import { Dialog, DialogContent, DialogHeader, DialogTitle } from "@sdkwork/ui-pc-react";

export interface ProviderAccountFillInGuideProps {
  open: boolean;
  onOpenChange(open: boolean): void;
}

export function ProviderAccountFillInGuide(props: ProviderAccountFillInGuideProps) {
  return (
    <Dialog open={props.open} onOpenChange={props.onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Fill-in guide</DialogTitle>
        </DialogHeader>
        <div className="max-h-[60dvh] space-y-4 overflow-y-auto pr-1 text-sm text-[var(--sdk-color-text-secondary)]">
          <GuideSection title="Account basics">
            <ul className="list-disc space-y-1 pl-4 text-xs leading-relaxed">
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Account No</strong>{" "}
                — unique identifier for this account (e.g., stripe-live-primary). Cannot be changed
                after creation.
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Merchant ID</strong>{" "}
                — merchant/vendor id issued by the provider (Alipay PID, WeChat mch_id, Stripe
                acct_xxx).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Environment</strong>{" "}
                — development, sandbox, or production.
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Account Mode</strong>{" "}
                — Direct (self-connection) or Partner / ISV (sub-merchants under a partner account).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Status</strong>{" "}
                — create as Inactive, validate the credentials, then activate.
              </li>
            </ul>
          </GuideSection>

          <GuideSection title="Credentials — Alipay">
            <ul className="list-disc space-y-1 pl-4 text-xs leading-relaxed">
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">
                  Merchant Private Key
                </strong>{" "}
                — RSA2 application private key downloaded from Alipay Open Platform (key tool →
                generate key pair).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Alipay Public Key</strong>{" "}
                — the platform's public key shown in the app console.
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">App ID</strong>{" "}
                — from the application details on Alipay Open Platform (metadata section).
              </li>
            </ul>
          </GuideSection>

          <GuideSection title="Credentials — WeChat Pay">
            <ul className="list-disc space-y-1 pl-4 text-xs leading-relaxed">
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">
                  Merchant Private Key
                </strong>{" "}
                — apiclient_key.pem downloaded from the merchant platform (Account Center → API
                Security).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">API v3 Key</strong>{" "}
                — 32-character key configured in the merchant platform (API Security → APIv3 key
                setting).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">
                  Platform Certificate
                </strong>{" "}
                — WeChat Pay platform public certificate (downloadable from the merchant platform).
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">
                  Merchant Serial No
                </strong>{" "}
                — certificate serial number shown next to the API certificate (metadata section).
              </li>
            </ul>
          </GuideSection>

          <GuideSection title="Credentials — Stripe">
            <ul className="list-disc space-y-1 pl-4 text-xs leading-relaxed">
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Secret Key</strong>{" "}
                — sk_live_... / sk_test_... from Dashboard → Developers → API keys.
              </li>
              <li>
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">
                  Webhook Signing Secret
                </strong>{" "}
                — whsec_... from the webhook endpoint details page.
              </li>
            </ul>
          </GuideSection>

          <GuideSection title="Notes">
            <ul className="list-disc space-y-1 pl-4 text-xs leading-relaxed">
              <li>
                Credential values are write-only: after saving they are never shown again — the
                field displays <em>Configured</em>. Saving a replacement overwrites the stored
                value.
              </li>
              <li>
                Each credential field accepts pasted PEM text or a local file via the{" "}
                <strong className="font-medium text-[var(--sdk-color-text-primary)]">Upload file</strong>{" "}
                link below the input. The file is read in the browser only and is sent to the
                server with the rest of the form.
              </li>
              <li>
                Partner accounts manage sub-merchants (Alipay sub_appid / WeChat sub_mch_id /
                Stripe Connected Accounts) in the Sub-Merchants tab after creation.
              </li>
            </ul>
          </GuideSection>
        </div>
      </DialogContent>
    </Dialog>
  );
}

interface GuideSectionProps {
  title: string;
  children: React.ReactNode;
}

function GuideSection({ title, children }: GuideSectionProps) {
  return (
    <section>
      <h4 className="mb-1.5 text-xs font-semibold uppercase tracking-wider text-[var(--sdk-color-text-muted)]">
        {title}
      </h4>
      {children}
    </section>
  );
}

export interface ProviderAccountFillInGuideLinkProps {
  onClick(): void;
}

export function ProviderAccountFillInGuideLink(props: ProviderAccountFillInGuideLinkProps) {
  return (
    <button
      type="button"
      onClick={props.onClick}
      title="Open the fill-in guide"
      className="inline-flex items-center gap-1 text-xs font-medium text-[var(--sdk-color-brand-primary)] underline underline-offset-4 hover:text-[var(--sdk-color-brand-primary-hover)] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[var(--sdk-color-border-focus)] focus-visible:ring-offset-2 focus-visible:ring-offset-[var(--sdk-color-surface-canvas)]"
    >
      <CircleHelp className="h-3.5 w-3.5" aria-hidden="true" />
      Fill-in guide
    </button>
  );
}
