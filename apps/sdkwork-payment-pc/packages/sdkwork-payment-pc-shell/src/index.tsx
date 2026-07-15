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
    // C20 alignment: register SDK service provider and session token provider on Shell mount.
    // 1. If an external appClient is injected (from host / federated router), prefer it;
    //    otherwise do not register and let the SDK throw an explicit error instead of
    //    silently falling back to an unavailable state.
    // 2. Session tokens are read from localStorage at runtime by the login flow;
    //    do not rely on vite define injection to avoid leaking tokens into the bundle.
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
