import {
  createContext,
  useContext,
  useMemo,
  type PropsWithChildren,
} from "react";
import { formatSdkworkPaymentCurrencyCny as formatSdkworkCurrencyCny } from "@sdkwork/payment-service";
import type {
  SdkworkPaymentProductType,
  SdkworkPaymentStatus,
} from "./payment";
import {
  createSdkworkPaymentMessages,
  normalizeSdkworkPaymentLocale,
  type SdkworkPaymentMessages,
  type SdkworkPaymentMessagesOverrides,
} from "./payment-copy";

export interface SdkworkPaymentIntlValue {
  copy: SdkworkPaymentMessages;
  formatCurrencyCny: (value: number | null | undefined) => string;
  formatPaginationSummary: (pageInfo: {
    page: number;
    pageSize: number;
    totalItems: number;
    totalPages: number;
  }) => string;
  formatPaymentSummary: (paymentId: string | number) => string;
  formatPollingText: (seconds: number | null | undefined, needQuery?: boolean) => string;
  formatProductType: (value: string | null | undefined) => string;
  formatRecommendedProductType: (value: string | null | undefined) => string;
  formatStatus: (status: string | null | undefined) => string;
  formatTimestamp: (value: string | undefined) => string;
  locale: string;
}

export interface SdkworkPaymentIntlProviderProps extends PropsWithChildren {
  locale?: string | null;
  messages?: SdkworkPaymentMessagesOverrides;
}

function interpolateSdkworkPaymentTemplate(
  template: string,
  values: Record<string, string>,
): string {
  return Object.entries(values).reduce(
    (output, [key, value]) => output.replaceAll(`{${key}}`, value),
    template,
  );
}

function normalizeProductTypeKey(
  value: string | null | undefined,
): keyof SdkworkPaymentMessages["productType"] {
  const normalized = String(value || "").trim().toLowerCase().replaceAll("-", "_");

  if (
    normalized === "app"
    || normalized === "h5"
    || normalized === "jsapi"
    || normalized === "miniapp"
    || normalized === "native"
    || normalized === "pc"
  ) {
    return normalized;
  }

  if (normalized === "online_bank") {
    return "onlineBank";
  }

  return "unknown";
}

function normalizeStatusKey(
  value: string | null | undefined,
): SdkworkPaymentStatus {
  const normalized = String(value || "").trim().toLowerCase();

  if (
    normalized === "closed"
    || normalized === "default"
    || normalized === "failed"
    || normalized === "pending"
    || normalized === "success"
    || normalized === "timeout"
  ) {
    return normalized;
  }

  return "unknown";
}

function createSdkworkPaymentIntlValue(
  locale?: string | null,
  overrides?: SdkworkPaymentMessagesOverrides,
): SdkworkPaymentIntlValue {
  const resolvedLocale = normalizeSdkworkPaymentLocale(locale);
  const copy = createSdkworkPaymentMessages(resolvedLocale, overrides);

  return {
    copy,
    formatCurrencyCny(value) {
      return formatSdkworkCurrencyCny(value, resolvedLocale);
    },
    formatPaginationSummary(pageInfo) {
      return interpolateSdkworkPaymentTemplate(copy.page.paginationSummary, {
        page: String(pageInfo.page),
        pageSize: String(pageInfo.pageSize),
        totalItems: String(pageInfo.totalItems),
        totalPages: String(pageInfo.totalPages),
        records: copy.page.paginationRecords,
      });
    },
    formatPaymentSummary(paymentId) {
      return interpolateSdkworkPaymentTemplate(copy.format.paymentSummary, {
        id: String(paymentId),
      });
    },
    formatPollingText(seconds, needQuery = true) {
      if (!needQuery) {
        return copy.detail.pollingIdle;
      }

      return interpolateSdkworkPaymentTemplate(copy.format.pollingRequired, {
        value: String(seconds ?? 3),
      });
    },
    formatProductType(value) {
      return copy.productType[normalizeProductTypeKey(value)];
    },
    formatRecommendedProductType(value) {
      return interpolateSdkworkPaymentTemplate(copy.format.recommendedProductTypeValue, {
        value: copy.productType[normalizeProductTypeKey(value)],
      });
    },
    formatStatus(status) {
      return copy.status[normalizeStatusKey(status)];
    },
    formatTimestamp(value) {
      if (!value) {
        return copy.common.emptyValue;
      }

      const timestamp = new Date(value);
      if (Number.isNaN(timestamp.getTime())) {
        return value;
      }

      return new Intl.DateTimeFormat(resolvedLocale, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(timestamp);
    },
    locale: resolvedLocale,
  };
}

const DEFAULT_SDKWORK_PAYMENT_INTL = createSdkworkPaymentIntlValue();

const SdkworkPaymentIntlContext = createContext<SdkworkPaymentIntlValue>(
  DEFAULT_SDKWORK_PAYMENT_INTL,
);

export function SdkworkPaymentIntlProvider({
  children,
  locale,
  messages,
}: SdkworkPaymentIntlProviderProps) {
  const value = useMemo(
    () => createSdkworkPaymentIntlValue(locale, messages),
    [locale, messages],
  );

  return (
    <SdkworkPaymentIntlContext.Provider value={value}>
      {children}
    </SdkworkPaymentIntlContext.Provider>
  );
}

export function useSdkworkPaymentIntl(): SdkworkPaymentIntlValue {
  return useContext(SdkworkPaymentIntlContext);
}
