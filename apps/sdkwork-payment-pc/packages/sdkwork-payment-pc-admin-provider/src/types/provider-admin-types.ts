/**
 * Provider admin type definitions.
 *
 * Mirrors backend OpenAPI schemas in
 * apis/backend-api/payment/sdkwork-payment-backend-api.openapi.yaml:
 *   - ProviderAccount (direct + partner/ISV mode)
 *   - CreateProviderAccountCommand / UpdateProviderAccountCommand
 *   - ProviderAccountTestResult
 *   - CredentialRotateCommand
 *   - SubMerchant (Alipay sub_appid / WeChat sub_mch_id / Stripe Connected Account)
 *   - Certificate metadata
 *
 * All remote payloads are typed as `unknown` at the SDK boundary; these types are
 * controller-side projections consumed by React. Field names mirror the wire contract.
 */

import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import type { SdkworkPaymentBackendService } from "@sdkwork/payment-service";

export type PaymentProviderCode = "stripe" | "alipay" | "wechat_pay" | "sandbox";

export type PaymentProviderAccountMode = "direct" | "partner";

export type PaymentProviderEnvironment = "development" | "sandbox" | "production";

export type PaymentProviderAccountStatus =
  | "active"
  | "inactive"
  | "suspended"
  | "deprecated";

export type PaymentLastTestStatus = "success" | "failure" | "unknown";

export type PaymentSubMerchantStatus =
  | "active"
  | "inactive"
  | "suspended"
  | "deprecated";

export type PaymentCertificateKind =
  | "merchant_private_key"
  | "provider_public_key"
  | "platform_certificate"
  | "webhook_secret";

export type PaymentCertificateStatus =
  | "active"
  | "expired"
  | "revoked"
  | "pending_rotation";

export interface PaymentProviderCapabilities {
  readonly pay?: boolean;
  readonly refund?: boolean;
  readonly close?: boolean;
  readonly query?: boolean;
  readonly reconcile?: boolean;
  readonly download?: boolean;
  readonly [key: string]: boolean | undefined;
}

export interface PaymentProviderAccountView {
  readonly id: string;
  readonly accountNo: string;
  readonly providerCode: PaymentProviderCode;
  readonly merchantId?: string;
  readonly accountMode: PaymentProviderAccountMode;
  readonly partnerProviderAccountId?: string;
  readonly environment: PaymentProviderEnvironment;
  readonly countryCode?: string;
  readonly settlementCurrency: string;
  readonly hasPrimarySecret: boolean;
  readonly hasWebhookSecret: boolean;
  readonly hasCertificate: boolean;
  readonly credentialStorage: "database_encrypted" | "legacy_reference" | "none";
  readonly capabilities: PaymentProviderCapabilities;
  readonly status: PaymentProviderAccountStatus;
  readonly metadata: Record<string, unknown>;
  readonly certificateExpiresAt?: string;
  readonly lastTestedAt?: string;
  readonly lastTestStatus?: PaymentLastTestStatus;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface PaymentSubMerchantView {
  readonly id: string;
  readonly providerAccountId: string;
  readonly subMerchantNo: string;
  readonly subMerchantName?: string;
  readonly subAppId?: string;
  readonly subMchId?: string;
  readonly stripeConnectedAccountId?: string;
  readonly providerCode: PaymentProviderCode;
  readonly status: PaymentSubMerchantStatus;
  readonly metadata: Record<string, unknown>;
  readonly createdAt: string;
  readonly updatedAt: string;
}

export interface PaymentProviderAccountTestResult {
  readonly ok: boolean;
  readonly providerCode: PaymentProviderCode;
  readonly environment: PaymentProviderEnvironment;
  readonly pspResponseCode?: string;
  readonly pspResponseTimeMs?: number;
  readonly diagnostic?: string;
  readonly testedAt: string;
}

export interface PaymentProviderAccountDraft {
  readonly accountNo: string;
  readonly providerCode: PaymentProviderCode;
  readonly merchantId: string;
  readonly accountMode: PaymentProviderAccountMode;
  readonly partnerProviderAccountId?: string;
  readonly environment: PaymentProviderEnvironment;
  readonly countryCode: string;
  readonly settlementCurrency: string;
  readonly primarySecret: string;
  readonly webhookSecret?: string;
  readonly certificate?: string;
  readonly capabilities?: PaymentProviderCapabilities;
  readonly status?: PaymentProviderAccountStatus;
  readonly metadata?: Record<string, unknown>;
}

export interface PaymentProviderAccountUpdateDraft {
  readonly merchantId?: string;
  readonly accountMode?: PaymentProviderAccountMode;
  readonly partnerProviderAccountId?: string;
  readonly environment?: PaymentProviderEnvironment;
  readonly countryCode?: string;
  readonly settlementCurrency?: string;
  readonly primarySecret?: string;
  readonly webhookSecret?: string;
  readonly certificate?: string;
  readonly capabilities?: PaymentProviderCapabilities;
  readonly status?: PaymentProviderAccountStatus;
  readonly metadata?: Record<string, unknown>;
}

export interface PaymentSubMerchantDraft {
  readonly providerAccountId: string;
  readonly subMerchantNo: string;
  readonly providerCode: PaymentProviderCode;
  readonly subMerchantName?: string;
  readonly subAppId?: string;
  readonly subMchId?: string;
  readonly stripeConnectedAccountId?: string;
  readonly status?: PaymentSubMerchantStatus;
  readonly metadata?: Record<string, unknown>;
}

export interface PaymentSubMerchantUpdateDraft {
  readonly subMerchantName?: string;
  readonly subAppId?: string;
  readonly subMchId?: string;
  readonly stripeConnectedAccountId?: string;
  readonly status?: PaymentSubMerchantStatus;
  readonly metadata?: Record<string, unknown>;
}

export interface PaymentCredentialRotateDraft {
  readonly primarySecret: string;
  readonly webhookSecret?: string;
  readonly certificate?: string;
  readonly invalidatePrevious?: boolean;
}

export interface PaymentProviderAccountTestOptions {
  readonly environment?: PaymentProviderEnvironment;
  readonly dryRun?: boolean;
}

export interface PaymentProviderAdminResourceSnapshot {
  readonly providerAccounts: readonly PaymentProviderAccountView[];
  readonly subMerchants: readonly PaymentSubMerchantView[];
}

export type PaymentProviderAdminStatus =
  | "idle"
  | "loading"
  | "ready"
  | "saving"
  | "testing"
  | "error";

export interface PaymentProviderAdminState extends PaymentProviderAdminResourceSnapshot {
  readonly listPageInfo?: Partial<Record<keyof PaymentProviderAdminResourceSnapshot, SdkWorkPageInfo>>;
  readonly status: PaymentProviderAdminStatus;
  readonly lastError?: string;
  readonly lastTestResult?: PaymentProviderAccountTestResult;
  readonly lastRotatedAccountId?: string;
  readonly selectedProviderAccount?: PaymentProviderAccountView;
  readonly selectedSubMerchant?: PaymentSubMerchantView;
}

export interface PaymentProviderAdminController {
  getState(): PaymentProviderAdminState;
  subscribe(listener: () => void): () => void;
  load(): Promise<PaymentProviderAdminState>;
  loadMoreProviderAccounts(): Promise<readonly PaymentProviderAccountView[]>;
  loadMoreSubMerchants(providerAccountId?: string): Promise<readonly PaymentSubMerchantView[]>;
  selectProviderAccount(id?: string): PaymentProviderAccountView | undefined;
  selectSubMerchant(id?: string): PaymentSubMerchantView | undefined;
  createProviderAccount(draft: PaymentProviderAccountDraft): Promise<PaymentProviderAccountView>;
  updateProviderAccount(id: string, draft: PaymentProviderAccountUpdateDraft): Promise<PaymentProviderAccountView>;
  testProviderAccount(id: string, options?: PaymentProviderAccountTestOptions): Promise<PaymentProviderAccountTestResult>;
  rotateProviderAccountCredentials(id: string, draft: PaymentCredentialRotateDraft): Promise<PaymentProviderAccountView>;
  createSubMerchant(draft: PaymentSubMerchantDraft): Promise<PaymentSubMerchantView>;
  updateSubMerchant(id: string, draft: PaymentSubMerchantUpdateDraft): Promise<PaymentSubMerchantView>;
  deleteSubMerchant(id: string): Promise<void>;
}

export interface CreatePaymentProviderAdminControllerInput {
  readonly service: SdkworkPaymentBackendService;
}
