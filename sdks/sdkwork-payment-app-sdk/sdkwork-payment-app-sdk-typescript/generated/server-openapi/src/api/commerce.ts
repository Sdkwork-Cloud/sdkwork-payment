import { appApiPath } from './paths';
import type { HttpClient } from '../http/client';

import type { CreatePaymentCommand, CreatePaymentIntentCommand, CreateRefundCommand, PageInfo, Payment, PaymentAttempt, PaymentIntent, PaymentMethod, PaymentRecord, PaymentStatistics, ReconcilePaymentCommand, Refund, SdkWorkCommandData } from '../types';


export interface CommerceRefundsListParams {
  page?: number;
  pageSize?: number;
  status?: string;
}

export class CommerceRefundsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List refunds. */
  async list(params?: CommerceRefundsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'status', value: params?.status, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(appApiPath(`/refunds`), query));
  }

/** Create a refund. */
  async create(body: CreateRefundCommand): Promise<Refund> {
    return this.client.post<Refund>(appApiPath(`/refunds`), body, undefined, undefined, 'application/json');
  }

/** Retrieve a refund. */
  async retrieve(refundId: string): Promise<Refund> {
    return this.client.get<Refund>(appApiPath(`/refunds/${serializePathParameter(refundId, { name: 'refundId', style: 'simple', explode: false })}`));
  }
}

export class CommercePaymentsStatusOutTradeNoApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve payment status by provider trade number. */
  async retrieve(outTradeNo: string): Promise<PaymentRecord> {
    return this.client.get<PaymentRecord>(appApiPath(`/payments/status/out_trade_no/${serializePathParameter(outTradeNo, { name: 'outTradeNo', style: 'simple', explode: false })}`));
  }
}

export class CommercePaymentsStatusApi {
  private client: HttpClient;
  public readonly outTradeNo: CommercePaymentsStatusOutTradeNoApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.outTradeNo = new CommercePaymentsStatusOutTradeNoApi(client);
  }


/** Retrieve payment status. */
  async retrieve(paymentId: string): Promise<PaymentRecord> {
    return this.client.get<PaymentRecord>(appApiPath(`/payments/status/${serializePathParameter(paymentId, { name: 'paymentId', style: 'simple', explode: false })}`));
  }
}

export class CommercePaymentsCheckoutApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve payment checkout data. */
  async retrieve(paymentId: string): Promise<Payment> {
    return this.client.get<Payment>(appApiPath(`/payments/checkout/${serializePathParameter(paymentId, { name: 'paymentId', style: 'simple', explode: false })}`));
  }
}

export class CommercePaymentsStatisticsSummaryApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve the payment statistics summary. */
  async retrieve(): Promise<PaymentStatistics> {
    return this.client.get<PaymentStatistics>(appApiPath(`/payments/statistics/summary`));
  }
}

export class CommercePaymentsStatisticsApi {
  private client: HttpClient;
  public readonly summary: CommercePaymentsStatisticsSummaryApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.summary = new CommercePaymentsStatisticsSummaryApi(client);
  }

}

export class CommercePaymentsAttemptsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Retrieve a payment attempt. */
  async retrieve(paymentAttemptId: string): Promise<PaymentAttempt> {
    return this.client.get<PaymentAttempt>(appApiPath(`/payments/attempts/${serializePathParameter(paymentAttemptId, { name: 'paymentAttemptId', style: 'simple', explode: false })}`));
  }
}

export interface CommercePaymentsRecordsListParams {
  page?: number;
  pageSize?: number;
  orderId?: string;
}

export class CommercePaymentsRecordsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List payment records. */
  async list(params?: CommercePaymentsRecordsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'order_id', value: params?.orderId, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(appApiPath(`/payments/records`), query));
  }

/** Retrieve a payment record. */
  async retrieve(paymentId: string): Promise<PaymentRecord> {
    return this.client.get<PaymentRecord>(appApiPath(`/payments/records/${serializePathParameter(paymentId, { name: 'paymentId', style: 'simple', explode: false })}`));
  }
}

export interface CommercePaymentsMethodsListParams {
  page?: number;
  pageSize?: number;
  clientType?: string;
}

export class CommercePaymentsMethodsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** List available payment methods. */
  async list(params?: CommercePaymentsMethodsListParams): Promise<Record<string, unknown>> {
    const query = buildQueryString([
      { name: 'page', value: params?.page, style: 'form', explode: true, allowReserved: false },
      { name: 'page_size', value: params?.pageSize, style: 'form', explode: true, allowReserved: false },
      { name: 'client_type', value: params?.clientType, style: 'form', explode: true, allowReserved: false },
    ]);
    return this.client.get<Record<string, unknown>>(appendQueryString(appApiPath(`/payments/methods`), query));
  }
}

export class CommercePaymentsIntentsAttemptsApi {
  private client: HttpClient;

  constructor(client: HttpClient) {
    this.client = client;
  }


/** Create a payment attempt. */
  async create(paymentIntentId: string): Promise<PaymentAttempt> {
    return this.client.post<PaymentAttempt>(appApiPath(`/payments/intents/${serializePathParameter(paymentIntentId, { name: 'paymentIntentId', style: 'simple', explode: false })}/attempts`));
  }
}

export class CommercePaymentsIntentsApi {
  private client: HttpClient;
  public readonly attempts: CommercePaymentsIntentsAttemptsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.attempts = new CommercePaymentsIntentsAttemptsApi(client);
  }


/** Create a payment intent. */
  async create(body: CreatePaymentIntentCommand): Promise<PaymentIntent> {
    return this.client.post<PaymentIntent>(appApiPath(`/payments/intents`), body, undefined, undefined, 'application/json');
  }

/** Retrieve a payment intent. */
  async retrieve(paymentIntentId: string): Promise<PaymentIntent> {
    return this.client.get<PaymentIntent>(appApiPath(`/payments/intents/${serializePathParameter(paymentIntentId, { name: 'paymentIntentId', style: 'simple', explode: false })}`));
  }

/** Cancel a payment intent. */
  async cancel(paymentIntentId: string): Promise<SdkWorkCommandData> {
    return this.client.post<SdkWorkCommandData>(appApiPath(`/payments/intents/${serializePathParameter(paymentIntentId, { name: 'paymentIntentId', style: 'simple', explode: false })}/cancel`));
  }
}

export class CommercePaymentsApi {
  private client: HttpClient;
  public readonly intents: CommercePaymentsIntentsApi;
  public readonly methods: CommercePaymentsMethodsApi;
  public readonly records: CommercePaymentsRecordsApi;
  public readonly attempts: CommercePaymentsAttemptsApi;
  public readonly statistics: CommercePaymentsStatisticsApi;
  public readonly checkout: CommercePaymentsCheckoutApi;
  public readonly status: CommercePaymentsStatusApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.intents = new CommercePaymentsIntentsApi(client);
    this.methods = new CommercePaymentsMethodsApi(client);
    this.records = new CommercePaymentsRecordsApi(client);
    this.attempts = new CommercePaymentsAttemptsApi(client);
    this.statistics = new CommercePaymentsStatisticsApi(client);
    this.checkout = new CommercePaymentsCheckoutApi(client);
    this.status = new CommercePaymentsStatusApi(client);
  }


/** Create a payment. */
  async create(body: CreatePaymentCommand): Promise<Payment> {
    return this.client.post<Payment>(appApiPath(`/payments`), body, undefined, undefined, 'application/json');
  }

/** Resolve the latest local payment record. */
  async reconcile(body: ReconcilePaymentCommand): Promise<PaymentRecord> {
    return this.client.post<PaymentRecord>(appApiPath(`/payments/reconcile`), body, undefined, undefined, 'application/json');
  }

/** Close a payment. */
  async close(paymentId: string): Promise<SdkWorkCommandData> {
    return this.client.post<SdkWorkCommandData>(appApiPath(`/payments/${serializePathParameter(paymentId, { name: 'paymentId', style: 'simple', explode: false })}/close`));
  }
}

export class CommerceApi {
  private client: HttpClient;
  public readonly payments: CommercePaymentsApi;
  public readonly refunds: CommerceRefundsApi;

  constructor(client: HttpClient) {
    this.client = client;
    this.payments = new CommercePaymentsApi(client);
    this.refunds = new CommerceRefundsApi(client);
  }

}

export function createCommerceApi(client: HttpClient): CommerceApi {
  return new CommerceApi(client);
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
