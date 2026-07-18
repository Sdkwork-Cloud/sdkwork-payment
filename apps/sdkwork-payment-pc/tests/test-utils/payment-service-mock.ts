import {
  configureSdkworkPaymentSessionTokenProvider,
  type SdkworkPaymentAppService,
  type SdkworkPaymentSessionTokens,
} from "@sdkwork/payment-service";

type DeepPartial<T> = {
  [K in keyof T]?: T[K] extends (...args: infer TArgs) => infer TReturn
    ? (...args: TArgs) => TReturn
    : DeepPartial<T[K]>;
};

export function createPaymentAppServiceMock(
  overrides: DeepPartial<SdkworkPaymentAppService> = {},
): SdkworkPaymentAppService {
  const base: SdkworkPaymentAppService = {
    payments: createMissingPaymentsTree(),
  };
  return mergePaymentAppService(base, overrides);
}

export function configurePaymentServiceMockSession(
  tokens: SdkworkPaymentSessionTokens = { authToken: "payment-auth-token" },
): void {
  configureSdkworkPaymentSessionTokenProvider(() => tokens);
}

export function resetPaymentServiceMockSession(): void {
  configureSdkworkPaymentSessionTokenProvider(null);
}

function createMissingPaymentsTree(): SdkworkPaymentAppService["payments"] {
  const tree: Record<string, unknown> = {};
  for (const method of [
    "close",
    "create",
    "methods.list",
    "statistics.summary.retrieve",
    "records.list",
    "records.retrieve",
    "status.retrieve",
    "status.outTradeNo.retrieve",
    "reconcile",
  ]) {
    addMissingMethod(tree, method);
  }
  return tree as SdkworkPaymentAppService["payments"];
}

function addMissingMethod(root: Record<string, unknown>, method: string): void {
  let node = root;
  const segments = method.split(".");
  for (const segment of segments.slice(0, -1)) {
    if (!node[segment] || typeof node[segment] === "function") {
      node[segment] = {};
    }
    node = node[segment] as Record<string, unknown>;
  }
  node[segments.at(-1)!] = async () => {
    throw new Error(`Missing payment service test method: payments.${method}`);
  };
}

function mergePaymentAppService<T>(base: T, overrides: DeepPartial<T>): T {
  for (const [key, value] of Object.entries(overrides as Record<string, unknown>)) {
    if (
      value &&
      typeof value === "object" &&
      !Array.isArray(value) &&
      typeof (base as Record<string, unknown>)[key] === "object"
    ) {
      mergePaymentAppService((base as Record<string, unknown>)[key], value as DeepPartial<unknown>);
    } else {
      (base as Record<string, unknown>)[key] = value;
    }
  }
  return base;
}
