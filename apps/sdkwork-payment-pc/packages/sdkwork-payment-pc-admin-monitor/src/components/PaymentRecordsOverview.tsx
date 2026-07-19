import {
  AlertTriangle,
  CheckCircle2,
  CircleDollarSign,
  ReceiptText,
} from "lucide-react";
import { StatCard } from "@sdkwork/ui-pc-react";
import { formatAdminAmount } from "@sdkwork/payment-pc-admin-core";
import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import { usePaymentRecordsMessages } from "../i18n";
import type { PaymentIntentView } from "../types/monitor-admin-types";

export interface PaymentRecordsOverviewProps {
  intents: readonly PaymentIntentView[];
  pageInfo?: SdkWorkPageInfo;
}

interface CurrencyVolume {
  currencyCode: string;
  total: number;
}

const COMPACT_METRIC_CLASS = "rounded-md max-sm:[&_[data-slot=stat-card-description]]:!hidden max-sm:[&_[data-slot=stat-card-value]]:!text-2xl";
const COMPACT_METRIC_WITHOUT_CHANGE_CLASS = `${COMPACT_METRIC_CLASS} max-sm:[&_[data-slot=stat-card-body]]:!hidden`;

function collectSuccessfulVolume(intents: readonly PaymentIntentView[]): CurrencyVolume[] {
  const totals = new Map<string, number>();
  for (const intent of intents) {
    if (intent.status !== "succeeded") {
      continue;
    }
    const amount = Number(intent.amount);
    if (!Number.isFinite(amount)) {
      continue;
    }
    const currencyCode = intent.currencyCode || "CNY";
    totals.set(currencyCode, (totals.get(currencyCode) ?? 0) + amount);
  }
  return Array.from(totals, ([currencyCode, total]) => ({ currencyCode, total }));
}

export function PaymentRecordsOverview({ intents, pageInfo }: PaymentRecordsOverviewProps) {
  const messages = usePaymentRecordsMessages();
  const succeededCount = intents.filter((intent) => intent.status === "succeeded").length;
  const exceptionCount = intents.filter(
    (intent) => intent.status === "failed" || intent.status === "canceled",
  ).length;
  const successRate = intents.length > 0 ? (succeededCount / intents.length) * 100 : 0;
  const currencyVolumes = collectSuccessfulVolume(intents);
  const successfulVolume = currencyVolumes.length === 1
    ? formatAdminAmount(currencyVolumes[0]?.total, currencyVolumes[0]?.currencyCode)
    : currencyVolumes.length > 1
      ? messages.metrics.multipleCurrencies(currencyVolumes.length)
      : "--";
  const successfulVolumeDescription = currencyVolumes.length > 1
    ? currencyVolumes
        .map(({ currencyCode, total }) => formatAdminAmount(total, currencyCode))
        .join(" · ")
    : messages.metrics.successfulVolumeDescription;

  return (
    <section
      aria-label={messages.metrics.currentResultSet}
      className="grid grid-cols-2 gap-3 xl:!grid-cols-4"
      data-slot="payment-records-overview"
    >
      <StatCard
        className={COMPACT_METRIC_WITHOUT_CHANGE_CLASS}
        description={messages.metrics.loadedDescription(intents.length)}
        icon={<ReceiptText aria-hidden="true" className="h-5 w-5" />}
        label={messages.metrics.recordsLabel}
        value={pageInfo?.totalItems ?? intents.length.toLocaleString()}
      />
      <StatCard
        className={COMPACT_METRIC_WITHOUT_CHANGE_CLASS}
        description={successfulVolumeDescription}
        icon={<CircleDollarSign aria-hidden="true" className="h-5 w-5" />}
        label={messages.metrics.successfulVolumeLabel}
        value={successfulVolume}
      />
      <StatCard
        change={`${succeededCount}/${intents.length}`}
        changeTone={successRate >= 90 ? "success" : successRate >= 70 ? "warning" : "danger"}
        className={COMPACT_METRIC_CLASS}
        description={messages.metrics.successRateDescription}
        icon={<CheckCircle2 aria-hidden="true" className="h-5 w-5" />}
        label={messages.metrics.successRateLabel}
        value={`${successRate.toFixed(intents.length > 0 ? 1 : 0)}%`}
      />
      <StatCard
        changeTone={exceptionCount > 0 ? "danger" : "success"}
        className={COMPACT_METRIC_WITHOUT_CHANGE_CLASS}
        description={messages.metrics.exceptionDescription}
        icon={<AlertTriangle aria-hidden="true" className="h-5 w-5" />}
        label={messages.metrics.exceptionLabel}
        value={exceptionCount.toLocaleString()}
      />
    </section>
  );
}
