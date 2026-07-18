import {
  APP_PAYMENT_METHOD_TREE,
  BACKEND_PAYMENT_METHOD_TREE,
  type ClientFromMethodTree,
  type PaymentAppSdkClient,
  type PaymentBackendSdkClient,
  type PaymentSdkMethod,
} from "@sdkwork/payment-sdk-ports";
import { formatCurrency as formatSdkworkCurrency } from "@sdkwork/utils";

// 重新导出 SDK 客户端类型，避免消费方需要直接依赖 @sdkwork/payment-sdk-ports。
export type { PaymentAppSdkClient, PaymentBackendSdkClient } from "@sdkwork/payment-sdk-ports";

type ServiceTemplate = { readonly [key: string]: true | ServiceTemplate };

export type SdkworkPaymentPaymentsService = ClientFromMethodTree<
  (typeof APP_PAYMENT_METHOD_TREE)["payments"]
>;

// Backend 服务：对齐 BACKEND_PAYMENT_METHOD_TREE 的全部资源。
// Admin UI 通过 `service.backend.*` 访问 provider accounts / sub merchants /
// certificates / dev config / webhook events / reconciliation 等后台能力。
export type SdkworkPaymentBackendService = ClientFromMethodTree<
  typeof BACKEND_PAYMENT_METHOD_TREE
>;

export type SdkworkPaymentAppService = {
  payments: SdkworkPaymentPaymentsService;
  backend?: SdkworkPaymentBackendService;
};

export type SdkworkPaymentAppServiceProvider = () => SdkworkPaymentAppService;

let sdkworkPaymentAppServiceProvider: SdkworkPaymentAppServiceProvider | null = null;

export interface SdkworkPaymentSessionTokens {
  accessToken?: string;
  authToken?: string;
  refreshToken?: string;
}

export type SdkworkPaymentSessionTokenProvider = () => SdkworkPaymentSessionTokens;

let sdkworkPaymentSessionTokenProvider: SdkworkPaymentSessionTokenProvider = () => ({});

export interface CreateSdkworkPaymentAppServiceInput {
  appClient: PaymentAppSdkClient;
  // 可选 backend SDK 客户端。Admin 场景下由 bootstrap / shell 注入；
  // 纯 C 端场景下省略，service.backend 为 undefined。
  backendClient?: PaymentBackendSdkClient | null;
}

export interface SdkworkPaymentResponseEnvelope<T> {
  code?: number | string;
  data?: T;
  message?: string;
  msg?: string;
  traceId?: string;
}

export type SdkworkMediaKind =
  | "archive"
  | "audio"
  | "document"
  | "image"
  | "model"
  | "other"
  | "video";

export type SdkworkMediaSource =
  | "data_url"
  | "external_url"
  | "generated"
  | "object_storage"
  | "provider_asset";

export interface SdkworkMediaResource {
  kind: SdkworkMediaKind;
  publicUrl?: string;
  source: SdkworkMediaSource;
  url?: string;
  [key: string]: unknown;
}

export function configureSdkworkPaymentAppServiceProvider(
  provider: SdkworkPaymentAppServiceProvider | null,
): void {
  sdkworkPaymentAppServiceProvider = provider;
}

export function configureSdkworkPaymentSessionTokenProvider(
  provider: SdkworkPaymentSessionTokenProvider | null,
): void {
  sdkworkPaymentSessionTokenProvider = provider ?? (() => ({}));
}

export function getSdkworkPaymentService(): SdkworkPaymentAppService {
  if (!sdkworkPaymentAppServiceProvider) {
    throw new Error(
      "SDKWork payment service provider is not configured. Call configureSdkworkPaymentAppServiceProvider() from payment PC bootstrap.",
    );
  }
  return sdkworkPaymentAppServiceProvider();
}

// Admin 场景专用：返回 backend 服务，若未注入 backend SDK 客户端则抛出明确错误。
export function getSdkworkPaymentBackendService(): SdkworkPaymentBackendService {
  const service = getSdkworkPaymentService();
  if (!service.backend) {
    throw new Error(
      "SDKWork payment backend service is not configured. Call configureSdkworkPaymentAppServiceProvider() with a backendClient from payment PC admin bootstrap.",
    );
  }
  return service.backend;
}

export function hasSdkworkPaymentBackendService(): boolean {
  const provider = sdkworkPaymentAppServiceProvider;
  if (!provider) {
    return false;
  }
  return Boolean(provider().backend);
}

export function getSdkworkPaymentSessionTokens(): SdkworkPaymentSessionTokens {
  const tokens = sdkworkPaymentSessionTokenProvider();
  return {
    accessToken: normalizeSessionToken(tokens.accessToken),
    authToken: normalizeSessionToken(tokens.authToken),
    refreshToken: normalizeSessionToken(tokens.refreshToken),
  };
}

export function hasSdkworkPaymentSession(): boolean {
  const tokens = getSdkworkPaymentSessionTokens();
  return Boolean(normalizeSessionToken(tokens.authToken) || normalizeSessionToken(tokens.accessToken));
}

export function requireSdkworkPaymentSession(message = "Authentication required"): void {
  if (!hasSdkworkPaymentSession()) {
    throw new Error(message);
  }
}

export function createSdkworkPaymentAppService(
  input: CreateSdkworkPaymentAppServiceInput,
): SdkworkPaymentAppService {
  const service: SdkworkPaymentAppService = {
    payments: buildServiceTree<SdkworkPaymentPaymentsService>(
      APP_PAYMENT_METHOD_TREE.payments,
      input.appClient.commerce.payments,
      ["commerce", "payments"],
    ),
  };

  // Admin 场景：注入 backend SDK 客户端后，构建 backend 服务树。
  // 纯 C 端场景下 backendClient 为 undefined，service.backend 不存在。
  if (input.backendClient) {
    service.backend = buildServiceTree<SdkworkPaymentBackendService>(
      BACKEND_PAYMENT_METHOD_TREE,
      input.backendClient.payments,
      ["payments"],
    );
  }

  return service;
}

export function createSdkworkPaymentBackendService(
  backendClient: PaymentBackendSdkClient,
): SdkworkPaymentBackendService {
  return buildServiceTree<SdkworkPaymentBackendService>(
    BACKEND_PAYMENT_METHOD_TREE,
    backendClient.payments,
    ["payments"],
  );
}

/**
 * C16/C17 对齐：RFC 9457 Problem+json 错误响应。
 *
 * 后端所有错误响应统一使用 `application/problem+json`，包含
 * `type/title/status/detail/code/traceId` 字段。前端 MUST 识别此结构并抛出
 * 结构化错误，而非静默吞掉或将 problem 对象误判为成功 data。
 */
export interface SdkworkPaymentProblemDetail {
  type?: string;
  title?: string;
  status?: number;
  detail?: string;
  code?: string;
  traceId?: string;
  errors?: unknown[];
  [key: string]: unknown;
}

export class SdkworkPaymentProblemError extends Error {
  readonly problem: SdkworkPaymentProblemDetail;
  readonly statusCode: number | undefined;
  readonly errorCode: string | undefined;
  readonly traceId: string | undefined;

  constructor(problem: SdkworkPaymentProblemDetail, fallbackMessage: string) {
    const message = String(problem.detail || problem.title || fallbackMessage).trim();
    super(message);
    this.name = "SdkworkPaymentProblemError";
    this.problem = problem;
    this.statusCode = problem.status;
    this.errorCode = problem.code;
    this.traceId = problem.traceId;
  }
}

function isProblemDetail(value: unknown): value is SdkworkPaymentProblemDetail {
  if (!value || typeof value !== "object") {
    return false;
  }
  const record = value as Record<string, unknown>;
  // RFC 9457 Problem+json 至少包含 type 和 title 字段
  return typeof record.type === "string" && typeof record.title === "string";
}

export function unwrapSdkworkPaymentResponse<T>(value: unknown, fallbackMessage = "Request failed."): T {
  if (!value || typeof value !== "object") {
    return value as T;
  }

  // C16 对齐：优先检测 RFC 9457 Problem+json 错误响应，避免将 problem 对象
  // 误判为成功 data 静默返回。
  if (isProblemDetail(value)) {
    throw new SdkworkPaymentProblemError(value, fallbackMessage);
  }

  if (!("data" in value) && !("code" in value)) {
    return value as T;
  }
  const envelope = value as SdkworkPaymentResponseEnvelope<T>;
  if (!isSuccessCode(envelope.code)) {
    throw new Error(String(envelope.message || envelope.msg || fallbackMessage).trim());
  }
  return (envelope.data ?? null) as T;
}

export function toSdkworkPaymentOptionalString(value: unknown): string | undefined {
  const normalized = typeof value === "string" ? value.trim() : String(value ?? "").trim();
  return normalized || undefined;
}

export function toNullableSdkworkPaymentNumber(value: unknown): number | null {
  if (typeof value === "number" && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === "string" && value.trim()) {
    const parsed = Number(value);
    return Number.isFinite(parsed) ? parsed : null;
  }
  return null;
}

export function toSdkworkPaymentNumber(value: unknown, fallback = 0): number {
  return toNullableSdkworkPaymentNumber(value) ?? fallback;
}

export function formatSdkworkPaymentCurrencyCny(value: number | null | undefined, language = "en-US"): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return "--";
  }
  return formatSdkworkCurrency(value, "CNY", language) ?? "--";
}

export function readSdkworkMediaResource(value: unknown): SdkworkMediaResource | undefined {
  if (!value || typeof value !== "object") {
    return undefined;
  }
  const record = value as Record<string, unknown>;
  const kind = typeof record.kind === "string" ? record.kind : undefined;
  const source = typeof record.source === "string" ? record.source : undefined;
  if (!kind || !source) {
    return undefined;
  }
  return { ...record, kind, source } as SdkworkMediaResource;
}

export function getSdkworkMediaDeliveryUrl(
  resource: Pick<SdkworkMediaResource, "publicUrl" | "url"> | null | undefined,
): string | undefined {
  const publicUrl = toSdkworkPaymentOptionalString(resource?.publicUrl);
  const url = toSdkworkPaymentOptionalString(resource?.url);
  return publicUrl || url;
}

export function toExternalSdkworkPaymentMediaResource(
  value: string | null | undefined,
  kind: SdkworkMediaKind,
): SdkworkMediaResource | undefined {
  const url = toSdkworkPaymentOptionalString(value);
  return url
    ? {
        kind,
        publicUrl: url,
        source: url.startsWith("data:") ? "data_url" : "external_url",
        url,
      }
    : undefined;
}

export function toExternalSdkworkMediaResource(
  value: string | null | undefined,
  kind: SdkworkMediaKind,
): SdkworkMediaResource | undefined {
  return toExternalSdkworkPaymentMediaResource(value, kind);
}

function buildServiceTree<TService>(
  template: ServiceTemplate,
  client: unknown,
  missingPathPrefix: readonly string[],
  servicePath: readonly string[] = [],
): TService {
  const service: Record<string, unknown> = {};
  for (const [key, marker] of Object.entries(template)) {
    const nextServicePath = [...servicePath, key];
    if (marker === true) {
      const missingPath = [...missingPathPrefix, ...nextServicePath].join(".");
      service[key] = (...args: Parameters<PaymentSdkMethod>) =>
        callPayment(readMethod(client, nextServicePath), missingPath, ...args);
    } else {
      service[key] = buildServiceTree<Record<string, unknown>>(
        marker,
        client,
        missingPathPrefix,
        nextServicePath,
      );
    }
  }
  return service as TService;
}

function readMethod(root: unknown, path: readonly string[]): PaymentSdkMethod | undefined {
  let node: unknown = root;
  for (const segment of path) {
    if (!node || typeof node !== "object") {
      return undefined;
    }
    const parent = node;
    node = (parent as Record<string, unknown>)[segment];
    if (typeof node === "function") {
      return node.bind(parent) as PaymentSdkMethod;
    }
  }
  return typeof node === "function" ? (node as PaymentSdkMethod) : undefined;
}

async function callPayment(
  method: PaymentSdkMethod | undefined,
  name: string,
  ...args: Parameters<PaymentSdkMethod>
): Promise<unknown> {
  if (!method) {
    throw new Error(`Missing SDKWork payment SDK resource: ${name}`);
  }
  return method(...args);
}

function normalizeSessionToken(value: unknown): string | undefined {
  const normalized = typeof value === "string" ? value.trim() : "";
  return normalized || undefined;
}

function isSuccessCode(code: number | string | undefined): boolean {
  // API_SPEC.md §15: 成功响应 `code` 必须为数值 `0`。
  // 缺失 `code` 字段（undefined/null/""）仅允许出现在裸 `{ data }` 兼容场景，
  // 此时已在调用方通过 `!("data" in value) && !("code" in value)` 早返回处理。
  // 一旦出现 `code` 字段，MUST 严格等于 `0`；禁止 200/2000/SUCCESS 等历史值。
  if (code === undefined || code === null || code === "") {
    return true;
  }
  if (typeof code === "number") {
    return code === 0;
  }
  return String(code).trim() === "0";
}
