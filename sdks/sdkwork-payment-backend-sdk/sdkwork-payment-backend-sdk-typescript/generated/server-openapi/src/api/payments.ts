import { backendApiPath } from './paths';
import type { HttpClient } from '../http/client';

import type { Certificate, CreateCertificateCommand, CreatePaymentChannelCommand, CreatePaymentMethodCommand, CreateProviderAccountCommand, CreateReconciliationRunCommand, CreateRouteRuleCommand, CreateSubMerchantCommand, CredentialRotateCommand, PageInfo, PaymentAttempt, PaymentChannel, PaymentIntent, PaymentMethod, ProviderAccount, ProviderAccountTestCommand, ProviderAccountTestResult, ReconciliationRun, RouteRule, SandboxTriggerCommand, SdkWorkAsyncData, SdkWorkCommandData, SubMerchant, UpdatePaymentMethodCommand, UpdateProviderAccountCommand, UpdateRouteRuleCommand, UpdateSubMerchantCommand, WebhookEvent, WebhookEventsReplayRequest, WebhookSignatureTestCommand, WebhookSignatureTestResult } from '../types';


export interface PaymentsDevSandboxTriggerParams {
  idempotencyKey?: string;
}

export interface PaymentsDevWebhookSignatureTestParams {
  idempotencyKey?: string;
}

export class PaymentsDevApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Sandbox event trigger (dev config). */
  async sandboxTrigger(body: SandboxTriggerCommand, params?: PaymentsDevSandboxTriggerParams): Promise<SdkWorkAsyncData> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<SdkWorkAsyncData>(backendApiPath(`/payments/dev/sandbox_trigger`), body, undefined, requestHeaders, 'application/json');
  }

/** Webhook signature verification test (dev config). */
  async webhookSignatureTest(body: WebhookSignatureTestCommand, params?: PaymentsDevWebhookSignatureTestParams): Promise<WebhookSignatureTestResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<WebhookSignatureTestResult>(backendApiPath(`/payments/dev/webhook_signature_test`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsReconciliationRunsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: 'pending' | 'queued' | 'running' | 'succeeded' | 'failed' | 'canceled';
  providerCode?: string;
  providerAccountId?: string;
}

export interface PaymentsReconciliationRunsCreateParams {
  idempotencyKey?: string;
}

export class PaymentsReconciliationRunsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Reconciliation runs list. */
  async list(params?: PaymentsReconciliationRunsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'providerAccountId', value: params?.providerAccountId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/reconciliation_runs`), query));
  }

/** Reconciliation run create. */
  async create(body: CreateReconciliationRunCommand, params?: PaymentsReconciliationRunsCreateParams): Promise<ReconciliationRun> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<ReconciliationRun>(backendApiPath(`/payments/reconciliation_runs`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsWebhookEventsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: 'queued' | 'processing' | 'processed' | 'failed' | 'dead';
  providerCode?: string;
  eventType?: string;
}

export class PaymentsWebhookEventsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Webhook events list. */
  async list(params?: PaymentsWebhookEventsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'eventType', value: params?.eventType, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/webhook_events`), query));
  }

/** Webhook event replay. */
  async replay(eventId: string, body?: WebhookEventsReplayRequest): Promise<SdkWorkCommandData> {
    return this.client.post<SdkWorkCommandData>(backendApiPath(`/payments/webhook_events/${serializePathParameter(eventId, { name: 'eventId', style: 'simple', explode: false })}/replay`), body, undefined, undefined, 'application/json');
  }
}

export interface PaymentsAttemptsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: 'created' | 'pending' | 'processing' | 'succeeded' | 'failed' | 'canceled' | 'closed';
  providerCode?: string;
  paymentIntentId?: string;
}

export class PaymentsAttemptsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Payment attempts list. */
  async list(params?: PaymentsAttemptsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'paymentIntentId', value: params?.paymentIntentId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/attempts`), query));
  }
}

export interface PaymentsCertificatesListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  providerCode?: string;
  certificateType?: 'merchant_private_key' | 'provider_public_key' | 'platform_certificate' | 'webhook_secret';
  expiringWithinDays?: number;
}

export interface PaymentsCertificatesCreateParams {
  idempotencyKey?: string;
}

export class PaymentsCertificatesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Certificates list. */
  async list(params?: PaymentsCertificatesListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'certificateType', value: params?.certificateType, style: 'form', explode: true, allowReserved: false },
      { name: 'expiringWithinDays', value: params?.expiringWithinDays, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/certificates`), query));
  }

/** Certificate create (upload/register PEM). */
  async create(body: CreateCertificateCommand, params?: PaymentsCertificatesCreateParams): Promise<Certificate> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<Certificate>(backendApiPath(`/payments/certificates`), body, undefined, requestHeaders, 'application/json');
  }

/** Certificate retrieve. */
  async retrieve(certificateId: string): Promise<Certificate> {
    return this.client.get<Certificate>(backendApiPath(`/payments/certificates/${serializePathParameter(certificateId, { name: 'certificateId', style: 'simple', explode: false })}`));
  }

/** Certificate delete. */
  async delete(certificateId: string): Promise<void> {
    return this.client.delete<void>(backendApiPath(`/payments/certificates/${serializePathParameter(certificateId, { name: 'certificateId', style: 'simple', explode: false })}`));
  }
}

export interface PaymentsSubMerchantsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  providerAccountId?: string;
  providerCode?: string;
  status?: 'active' | 'inactive' | 'suspended' | 'deprecated';
}

export interface PaymentsSubMerchantsCreateParams {
  idempotencyKey?: string;
}

export interface PaymentsSubMerchantsUpdateParams {
  idempotencyKey?: string;
}

export class PaymentsSubMerchantsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Sub-merchants list (ISV/partner mode only). */
  async list(params?: PaymentsSubMerchantsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'providerAccountId', value: params?.providerAccountId, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/sub_merchants`), query));
  }

/** Sub-merchant create (ISV/partner mode only). */
  async create(body: CreateSubMerchantCommand, params?: PaymentsSubMerchantsCreateParams): Promise<SubMerchant> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<SubMerchant>(backendApiPath(`/payments/sub_merchants`), body, undefined, requestHeaders, 'application/json');
  }

/** Sub-merchant retrieve. */
  async retrieve(subMerchantId: string): Promise<SubMerchant> {
    return this.client.get<SubMerchant>(backendApiPath(`/payments/sub_merchants/${serializePathParameter(subMerchantId, { name: 'subMerchantId', style: 'simple', explode: false })}`));
  }

/** Sub-merchant update. */
  async update(subMerchantId: string, body: UpdateSubMerchantCommand, params?: PaymentsSubMerchantsUpdateParams): Promise<SubMerchant> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.patch<SubMerchant>(backendApiPath(`/payments/sub_merchants/${serializePathParameter(subMerchantId, { name: 'subMerchantId', style: 'simple', explode: false })}`), body, undefined, requestHeaders, 'application/json');
  }

/** Sub-merchant delete. */
  async delete(subMerchantId: string): Promise<void> {
    return this.client.delete<void>(backendApiPath(`/payments/sub_merchants/${serializePathParameter(subMerchantId, { name: 'subMerchantId', style: 'simple', explode: false })}`));
  }
}

export interface PaymentsRouteRulesListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: 'active' | 'inactive' | 'deprecated';
  channelId?: string;
}

export interface PaymentsRouteRulesCreateParams {
  idempotencyKey?: string;
}

export interface PaymentsRouteRulesUpdateParams {
  idempotencyKey?: string;
}

export class PaymentsRouteRulesApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Route rules list. */
  async list(params?: PaymentsRouteRulesListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'channelId', value: params?.channelId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/route_rules`), query));
  }

/** Route rule create. */
  async create(body: CreateRouteRuleCommand, params?: PaymentsRouteRulesCreateParams): Promise<RouteRule> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<RouteRule>(backendApiPath(`/payments/route_rules`), body, undefined, requestHeaders, 'application/json');
  }

/** Route rule update. */
  async update(routeRuleId: string, body: UpdateRouteRuleCommand, params?: PaymentsRouteRulesUpdateParams): Promise<RouteRule> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.patch<RouteRule>(backendApiPath(`/payments/route_rules/${serializePathParameter(routeRuleId, { name: 'routeRuleId', style: 'simple', explode: false })}`), body, undefined, requestHeaders, 'application/json');
  }

/** Route rule delete. */
  async delete(routeRuleId: string): Promise<void> {
    return this.client.delete<void>(backendApiPath(`/payments/route_rules/${serializePathParameter(routeRuleId, { name: 'routeRuleId', style: 'simple', explode: false })}`));
  }
}

export interface PaymentsChannelsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  providerCode?: string;
  sceneCode?: 'app' | 'web' | 'mini_program' | 'api';
  status?: 'active' | 'inactive' | 'deprecated';
}

export interface PaymentsChannelsCreateParams {
  idempotencyKey?: string;
}

export class PaymentsChannelsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Payment channels list. */
  async list(params?: PaymentsChannelsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'sceneCode', value: params?.sceneCode, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/channels`), query));
  }

/** Payment channel create. */
  async create(body: CreatePaymentChannelCommand, params?: PaymentsChannelsCreateParams): Promise<PaymentChannel> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<PaymentChannel>(backendApiPath(`/payments/channels`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsProviderAccountsCredentialsRotateParams {
  idempotencyKey?: string;
}

export class PaymentsProviderAccountsCredentialsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Provider account credential rotation. */
  async rotate(providerAccountId: string, body: CredentialRotateCommand, params?: PaymentsProviderAccountsCredentialsRotateParams): Promise<ProviderAccount> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<ProviderAccount>(backendApiPath(`/payments/provider_accounts/${serializePathParameter(providerAccountId, { name: 'providerAccountId', style: 'simple', explode: false })}/credentials/rotate`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsProviderAccountsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  providerCode?: 'stripe' | 'alipay' | 'wechat_pay' | 'sandbox';
  environment?: 'development' | 'sandbox' | 'production';
  accountMode?: 'direct' | 'partner';
  status?: 'active' | 'inactive' | 'suspended' | 'deprecated';
}

export interface PaymentsProviderAccountsCreateParams {
  idempotencyKey?: string;
}

export interface PaymentsProviderAccountsUpdateParams {
  idempotencyKey?: string;
}

export interface PaymentsProviderAccountsTestParams {
  idempotencyKey?: string;
}

export class PaymentsProviderAccountsApi {
  private client: HttpClient;
  public readonly credentials: PaymentsProviderAccountsCredentialsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.credentials = new PaymentsProviderAccountsCredentialsApi(client);
  }


/** Provider accounts list. */
  async list(params?: PaymentsProviderAccountsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'providerCode', value: params?.providerCode, style: 'form', explode: true, allowReserved: false },
      { name: 'environment', value: params?.environment, style: 'form', explode: true, allowReserved: false },
      { name: 'accountMode', value: params?.accountMode, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/provider_accounts`), query));
  }

/** Provider account create. */
  async create(body: CreateProviderAccountCommand, params?: PaymentsProviderAccountsCreateParams): Promise<ProviderAccount> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<ProviderAccount>(backendApiPath(`/payments/provider_accounts`), body, undefined, requestHeaders, 'application/json');
  }

/** Provider account update. */
  async update(providerAccountId: string, body: UpdateProviderAccountCommand, params?: PaymentsProviderAccountsUpdateParams): Promise<ProviderAccount> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.patch<ProviderAccount>(backendApiPath(`/payments/provider_accounts/${serializePathParameter(providerAccountId, { name: 'providerAccountId', style: 'simple', explode: false })}`), body, undefined, requestHeaders, 'application/json');
  }

/** Provider account credential connectivity test. */
  async test(providerAccountId: string, body?: ProviderAccountTestCommand, params?: PaymentsProviderAccountsTestParams): Promise<ProviderAccountTestResult> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<ProviderAccountTestResult>(backendApiPath(`/payments/provider_accounts/${serializePathParameter(providerAccountId, { name: 'providerAccountId', style: 'simple', explode: false })}/test`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsMethodsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: 'active' | 'inactive' | 'deprecated';
}

export interface PaymentsMethodsCreateParams {
  idempotencyKey?: string;
}

export interface PaymentsMethodsUpdateParams {
  idempotencyKey?: string;
}

export class PaymentsMethodsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Payment methods list. */
  async list(params?: PaymentsMethodsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/methods`), query));
  }

/** Payment method create. */
  async create(body: CreatePaymentMethodCommand, params?: PaymentsMethodsCreateParams): Promise<PaymentMethod> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.post<PaymentMethod>(backendApiPath(`/payments/methods`), body, undefined, requestHeaders, 'application/json');
  }

/** Payment method update. */
  async update(methodKey: string, body: UpdatePaymentMethodCommand, params?: PaymentsMethodsUpdateParams): Promise<PaymentMethod> {
    const requestHeaders = buildRequestHeaders(
      {
        'Idempotency-Key': { value: params?.idempotencyKey, style: 'simple', explode: false },
      },
      {}
    );
    return this.client.patch<PaymentMethod>(backendApiPath(`/payments/methods/${serializePathParameter(methodKey, { name: 'methodKey', style: 'simple', explode: false })}`), body, undefined, requestHeaders, 'application/json');
  }
}

export interface PaymentsIntentsListParams {
  page?: number;
  pageSize?: number;
  sort?: string;
  q?: string;
  status?: string;
  ownerUserId?: string;
  orderId?: string;
}

export class PaymentsIntentsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Payment intents list. */
  async list(params?: PaymentsIntentsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'sort', value: params?.sort, style: 'form', explode: true, allowReserved: false },
      { name: 'q', value: params?.q, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
      { name: 'ownerUserId', value: params?.ownerUserId, style: 'form', explode: true, allowReserved: false },
      { name: 'orderId', value: params?.orderId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(backendApiPath(`/payments/intents`), query));
  }

/** Payment intent retrieve. */
  async retrieve(paymentIntentId: string): Promise<PaymentIntent> {
    return this.client.get<PaymentIntent>(backendApiPath(`/payments/intents/${serializePathParameter(paymentIntentId, { name: 'paymentIntentId', style: 'simple', explode: false })}`));
  }
}

export class PaymentsApi {
  private client: HttpClient;
  public readonly intents: PaymentsIntentsApi;
  public readonly methods: PaymentsMethodsApi;
  public readonly providerAccounts: PaymentsProviderAccountsApi;
  public readonly channels: PaymentsChannelsApi;
  public readonly routeRules: PaymentsRouteRulesApi;
  public readonly subMerchants: PaymentsSubMerchantsApi;
  public readonly certificates: PaymentsCertificatesApi;
  public readonly attempts: PaymentsAttemptsApi;
  public readonly webhookEvents: PaymentsWebhookEventsApi;
  public readonly reconciliationRuns: PaymentsReconciliationRunsApi;
  public readonly dev: PaymentsDevApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.intents = new PaymentsIntentsApi(client);
    this.methods = new PaymentsMethodsApi(client);
    this.providerAccounts = new PaymentsProviderAccountsApi(client);
    this.channels = new PaymentsChannelsApi(client);
    this.routeRules = new PaymentsRouteRulesApi(client);
    this.subMerchants = new PaymentsSubMerchantsApi(client);
    this.certificates = new PaymentsCertificatesApi(client);
    this.attempts = new PaymentsAttemptsApi(client);
    this.webhookEvents = new PaymentsWebhookEventsApi(client);
    this.reconciliationRuns = new PaymentsReconciliationRunsApi(client);
    this.dev = new PaymentsDevApi(client);
  }

}

export function createPaymentsApi(client: HttpClient): PaymentsApi {
  return new PaymentsApi(client);
}

function appendQueryString(path: string, rawQueryString: string): string {
  const query = rawQueryString.replace(/^\?+/, '');
  if (!query) {
    return path;
  }
  return path.includes('?') ? `${path}&${query}` : `${path}?${query}`;
}

interface PathParameterSpec {
  name: string;
  style: string;
  explode: boolean;
}

function serializePathParameter(value: unknown, spec: PathParameterSpec): string {
  if (value === undefined || value === null) {
    return '';
  }

  const style = spec.style || 'simple';
  if (Array.isArray(value)) {
    return serializePathArray(spec.name, value, style, spec.explode);
  }
  if (typeof value === 'object') {
    return serializePathObject(spec.name, value as Record<string, unknown>, style, spec.explode);
  }
  return pathPrefix(spec.name, style, false) + encodePathValue(serializePathPrimitive(value));
}

function serializePathArray(name: string, values: unknown[], style: string, explode: boolean): string {
  const serialized = values
    .filter((item) => item !== undefined && item !== null)
    .map((item) => encodePathValue(serializePathPrimitive(item)));
  if (serialized.length === 0) {
    return pathPrefix(name, style, false);
  }
  if (style === 'matrix') {
    return explode
      ? serialized.map((item) => `;${name}=${item}`).join('')
      : `;${name}=${serialized.join(',')}`;
  }
  return pathPrefix(name, style, false) + serialized.join(explode ? '.' : ',');
}

function serializePathObject(name: string, value: Record<string, unknown>, style: string, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return pathPrefix(name, style, true);
  }
  if (style === 'matrix') {
    return explode
      ? entries.map(([key, entryValue]) => `;${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join('')
      : `;${name}=${entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',')}`;
  }
  const serialized = explode
    ? entries.map(([key, entryValue]) => `${encodePathValue(key)}=${encodePathValue(serializePathPrimitive(entryValue))}`).join(style === 'label' ? '.' : ',')
    : entries.flatMap(([key, entryValue]) => [encodePathValue(key), encodePathValue(serializePathPrimitive(entryValue))]).join(',');
  return pathPrefix(name, style, true) + serialized;
}

function pathPrefix(name: string, style: string, _objectValue: boolean): string {
  if (style === 'label') return '.';
  if (style === 'matrix') return `;${name}`;
  return '';
}

function encodePathValue(value: string): string {
  return encodeURIComponent(value);
}

function serializePathPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}
interface QueryParameterSpec {
  name: string;
  value: unknown;
  style: string;
  explode: boolean;
  allowReserved: boolean;
  contentType?: string;
}

function buildQueryString(parameters: QueryParameterSpec[]): string {
  const pairs: string[] = [];
  for (const parameter of parameters) {
    appendSerializedParameter(pairs, parameter);
  }
  return pairs.join('&');
}

function appendSerializedParameter(pairs: string[], parameter: QueryParameterSpec): void {
  if (parameter.value === undefined || parameter.value === null) {
    return;
  }

  if (parameter.contentType) {
    pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(JSON.stringify(parameter.value), parameter.allowReserved)}`);
    return;
  }

  const style = parameter.style || 'form';
  if (style === 'deepObject') {
    appendDeepObjectParameter(pairs, parameter.name, parameter.value, parameter.allowReserved);
    return;
  }

  if (Array.isArray(parameter.value)) {
    appendArrayParameter(pairs, parameter.name, parameter.value, style, parameter.explode, parameter.allowReserved);
    return;
  }

  if (typeof parameter.value === 'object') {
    appendObjectParameter(pairs, parameter.name, parameter.value as Record<string, unknown>, style, parameter.explode, parameter.allowReserved);
    return;
  }

  pairs.push(`${encodeQueryComponent(parameter.name)}=${encodeQueryValue(serializePrimitive(parameter.value), parameter.allowReserved)}`);
}

function appendArrayParameter(
  pairs: string[],
  name: string,
  value: unknown[],
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const values = value
    .filter((item) => item !== undefined && item !== null)
    .map((item) => serializePrimitive(item));
  if (values.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const item of values) {
      pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(item, allowReserved)}`);
    }
    return;
  }

  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(values.join(','), allowReserved)}`);
}

function appendObjectParameter(
  pairs: string[],
  name: string,
  value: Record<string, unknown>,
  style: string,
  explode: boolean,
  allowReserved: boolean,
): void {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (entries.length === 0) {
    return;
  }

  if (style === 'form' && explode) {
    for (const [key, entryValue] of entries) {
      pairs.push(`${encodeQueryComponent(key)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
    }
    return;
  }

  const serialized = entries.flatMap(([key, entryValue]) => [key, serializePrimitive(entryValue)]).join(',');
  pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serialized, allowReserved)}`);
}

function appendDeepObjectParameter(
  pairs: string[],
  name: string,
  value: unknown,
  allowReserved: boolean,
): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)) {
    pairs.push(`${encodeQueryComponent(name)}=${encodeQueryValue(serializePrimitive(value), allowReserved)}`);
    return;
  }

  for (const [key, entryValue] of Object.entries(value as Record<string, unknown>)) {
    if (entryValue === undefined || entryValue === null) {
      continue;
    }
    pairs.push(`${encodeQueryComponent(`${name}[${key}]`)}=${encodeQueryValue(serializePrimitive(entryValue), allowReserved)}`);
  }
}

function serializePrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (typeof value === 'object') {
    return JSON.stringify(value);
  }
  return String(value);
}

function encodeQueryComponent(value: string): string {
  return encodeURIComponent(value);
}

function encodeQueryValue(value: string, allowReserved: boolean): string {
  const encoded = encodeURIComponent(value);
  if (!allowReserved) {
    return encoded;
  }
  return encoded.replace(/%3A/gi, ':')
    .replace(/%2F/gi, '/')
    .replace(/%3F/gi, '?')
    .replace(/%23/gi, '#')
    .replace(/%5B/gi, '[')
    .replace(/%5D/gi, ']')
    .replace(/%40/gi, '@')
    .replace(/%21/gi, '!')
    .replace(/%24/gi, '$')
    .replace(/%26/gi, '&')
    .replace(/%27/gi, "'")
    .replace(/%28/gi, '(')
    .replace(/%29/gi, ')')
    .replace(/%2A/gi, '*')
    .replace(/%2B/gi, '+')
    .replace(/%2C/gi, ',')
    .replace(/%3B/gi, ';')
    .replace(/%3D/gi, '=');
}
function buildRequestHeaders(
  headers: Record<string, HeaderParameterSpec | undefined>,
  cookies: Record<string, HeaderParameterSpec | undefined> = {},
): Record<string, string> | undefined {
  const requestHeaders: Record<string, string> = {};

  for (const [name, parameter] of Object.entries(headers)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      requestHeaders[name] = serialized;
    }
  }

  const cookieHeader = buildCookieHeader(cookies);
  if (cookieHeader) {
    requestHeaders.Cookie = requestHeaders.Cookie
      ? `${requestHeaders.Cookie}; ${cookieHeader}`
      : cookieHeader;
  }

  return Object.keys(requestHeaders).length > 0 ? requestHeaders : undefined;
}

interface HeaderParameterSpec {
  value: unknown;
  style: string;
  explode: boolean;
  contentType?: string;
}

function buildCookieHeader(cookies: Record<string, HeaderParameterSpec | undefined>): string | undefined {
  const pairs: string[] = [];
  for (const [name, parameter] of Object.entries(cookies)) {
    const serialized = serializeParameterValue(parameter);
    if (serialized !== undefined) {
      pairs.push(`${encodeURIComponent(name)}=${encodeURIComponent(serialized)}`);
    }
  }
  return pairs.length > 0 ? pairs.join('; ') : undefined;
}

function serializeParameterValue(parameter: HeaderParameterSpec | undefined): string | undefined {
  const value = parameter?.value;
  if (value === undefined || value === null) {
    return undefined;
  }
  if (parameter?.contentType) {
    return JSON.stringify(value);
  }
  if (value instanceof Date) {
    return value.toISOString();
  }
  if (Array.isArray(value)) {
    return value.map((item) => serializeHeaderPrimitive(item)).join(',');
  }
  if (typeof value === 'object' && value !== null) {
    return serializeHeaderObject(value as Record<string, unknown>, parameter?.explode === true);
  }
  return serializeHeaderPrimitive(value);
}

function serializeHeaderObject(value: Record<string, unknown>, explode: boolean): string {
  const entries = Object.entries(value).filter(([, entryValue]) => entryValue !== undefined && entryValue !== null);
  if (explode) {
    return entries.map(([key, entryValue]) => `${key}=${serializeHeaderPrimitive(entryValue)}`).join(',');
  }
  return entries.flatMap(([key, entryValue]) => [key, serializeHeaderPrimitive(entryValue)]).join(',');
}

function serializeHeaderPrimitive(value: unknown): string {
  if (value instanceof Date) {
    return value.toISOString();
  }
  return String(value);
}
