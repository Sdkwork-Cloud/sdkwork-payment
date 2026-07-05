import {
  useEffect,
  useState,
  type ReactNode,
} from "react";
import {
  configureSdkworkPaymentAppServiceProvider,
  configureSdkworkPaymentSessionTokenProvider,
  createSdkworkPaymentAppService,
  type PaymentAppSdkClient,
  type SdkworkPaymentSessionTokens,
} from "@sdkwork/payment-service";
import { sdkworkPaymentPcRuntimeIdentity } from "@sdkwork/payment-pc-core";
import { SdkworkPaymentPage } from "@sdkwork/payment-pc-payment";

export interface PaymentAppShellProps {
  appClient?: PaymentAppSdkClient | null;
  sessionTokens?: SdkworkPaymentSessionTokens | (() => SdkworkPaymentSessionTokens);
  fallback?: ReactNode;
}

const SESSION_STORAGE_KEYS = {
  accessToken: "sdkwork.payment.accessToken",
  authToken: "sdkwork.payment.authToken",
  refreshToken: "sdkwork.payment.refreshToken",
} as const;

function readSessionToken(key: string): string | undefined {
  try {
    const value = window.localStorage.getItem(key);
    return value && value.trim() ? value.trim() : undefined;
  } catch {
    return undefined;
  }
}

function readSessionTokens(): SdkworkPaymentSessionTokens {
  return {
    accessToken: readSessionToken(SESSION_STORAGE_KEYS.accessToken),
    authToken: readSessionToken(SESSION_STORAGE_KEYS.authToken),
    refreshToken: readSessionToken(SESSION_STORAGE_KEYS.refreshToken),
  };
}

export function PaymentAppShell({
  appClient,
  sessionTokens,
  fallback,
}: PaymentAppShellProps = {}): ReactNode {
  const [isConfigured, setIsConfigured] = useState(false);

  useEffect(() => {
    // C20 对齐：在 Shell 挂载时注册 SDK 服务提供者与会话 token 提供者。
    // 1. 若外部已注入 appClient（来自宿主 / federated router），优先使用；
    //    否则不注册，由 SDK 抛出明确错误，避免静默回退到不可用状态。
    // 2. 会话 token 通过 localStorage 读取，运行时由登录流程写入；
    //    不再依赖 vite define 注入，避免 token 泄漏到 bundle。
    if (appClient) {
      configureSdkworkPaymentAppServiceProvider(() =>
        createSdkworkPaymentAppService({ appClient }),
      );
    }
    configureSdkworkPaymentSessionTokenProvider(
      typeof sessionTokens === "function" ? sessionTokens : readSessionTokens,
    );
    setIsConfigured(true);
  }, [appClient, sessionTokens]);

  return (
    <main className="payment-shell" data-app-key={sdkworkPaymentPcRuntimeIdentity.appKey}>
      {isConfigured ? <SdkworkPaymentPage /> : (fallback ?? null)}
    </main>
  );
}
