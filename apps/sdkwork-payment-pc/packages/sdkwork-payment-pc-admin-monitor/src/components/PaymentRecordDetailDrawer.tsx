import { Clock3 } from "lucide-react";
import {
  Badge,
  Button,
  DescriptionDetails,
  DescriptionItem,
  DescriptionList,
  DescriptionTerm,
  DetailDrawer,
  DetailDrawerMetric,
  DetailDrawerMetrics,
  DetailDrawerSection,
} from "@sdkwork/ui-pc-react";
import {
  CopyButton,
  formatAdminAmount,
  formatAdminTimestamp,
} from "@sdkwork/payment-pc-admin-core";
import { usePaymentRecordsMessages } from "../i18n";
import type { PaymentIntentDetail, PaymentStatus } from "../types/monitor-admin-types";
import {
  formatPaymentProvider,
  PAYMENT_STATUS_BADGE_VARIANT,
} from "./payment-record-presentation";

export interface PaymentRecordDetailDrawerProps {
  detail?: PaymentIntentDetail;
  onOpenChange(open: boolean): void;
  open: boolean;
}

function statusTone(status: PaymentStatus): "default" | "success" | "warning" | "danger" {
  if (status === "succeeded" || status === "refunded") {
    return "success";
  }
  if (status === "failed") {
    return "danger";
  }
  if (status === "pending" || status === "processing" || status === "refunding") {
    return "warning";
  }
  return "default";
}

export function PaymentRecordDetailDrawer({
  detail,
  onOpenChange,
  open,
}: PaymentRecordDetailDrawerProps) {
  const messages = usePaymentRecordsMessages();
  const attempts = detail?.attempts ?? [];

  return (
    <DetailDrawer
      actions={detail ? (
        <CopyButton
          label={messages.actions.copyIdentifier}
          title={messages.actions.copyIdentifier}
          value={detail.paymentIntentNo}
          variant="outline"
        />
      ) : undefined}
      description={detail?.orderId ? `${messages.detail.orderIdentifier}: ${detail.orderId}` : undefined}
      eyebrow={messages.detail.paymentRecord}
      footer={(
        <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
          {messages.actions.close}
        </Button>
      )}
      open={open}
      size="xl"
      summary={detail ? (
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Badge variant={PAYMENT_STATUS_BADGE_VARIANT[detail.status]}>
              {messages.status[detail.status]}
            </Badge>
            <Badge variant="outline">{formatPaymentProvider(detail.providerCode)}</Badge>
          </div>
          <strong className="text-base text-[var(--sdk-color-text-primary)]">
            {formatAdminAmount(detail.amount, detail.currencyCode)}
          </strong>
        </div>
      ) : undefined}
      title={detail?.paymentIntentNo ?? messages.detail.paymentRecord}
      onOpenChange={onOpenChange}
    >
      {detail ? (
        <>
          <DetailDrawerMetrics columns={3}>
            <DetailDrawerMetric
              label={messages.detail.amount}
              value={formatAdminAmount(detail.amount, detail.currencyCode)}
            />
            <DetailDrawerMetric
              label={messages.detail.status}
              tone={statusTone(detail.status)}
              value={messages.status[detail.status]}
            />
            <DetailDrawerMetric
              label={messages.detail.provider}
              value={formatPaymentProvider(detail.providerCode)}
            />
          </DetailDrawerMetrics>

          <div className="space-y-2">
            <h3 className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
              {messages.detail.customerAndOrder}
            </h3>
            <DescriptionList columns={2}>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.intentIdentifier}</DescriptionTerm>
                <DescriptionDetails className="break-all" mono>{detail.paymentIntentNo}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.orderIdentifier}</DescriptionTerm>
                <DescriptionDetails className="break-all" mono>{detail.orderId || "--"}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.ownerIdentifier}</DescriptionTerm>
                <DescriptionDetails className="break-all" mono>{detail.ownerUserId || "--"}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.method}</DescriptionTerm>
                <DescriptionDetails>{detail.paymentMethod || "--"}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.createdAt}</DescriptionTerm>
                <DescriptionDetails>{formatAdminTimestamp(detail.createdAt)}</DescriptionDetails>
              </DescriptionItem>
              <DescriptionItem>
                <DescriptionTerm>{messages.detail.updatedAt}</DescriptionTerm>
                <DescriptionDetails>{formatAdminTimestamp(detail.updatedAt)}</DescriptionDetails>
              </DescriptionItem>
            </DescriptionList>
          </div>

          <DetailDrawerSection
            description={messages.detail.attemptsDescription}
            title={messages.detail.attempts}
          >
            {attempts.length > 0 ? (
              <div className="divide-y divide-[var(--sdk-color-border-subtle)] border-y border-[var(--sdk-color-border-subtle)]">
                {attempts.map((attempt) => (
                  <div key={attempt.id} className="grid gap-3 py-3 sm:grid-cols-[minmax(0,1fr)_auto] sm:items-center">
                    <div className="min-w-0 space-y-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <span className="break-all font-mono text-sm font-medium">{attempt.attemptNo}</span>
                        <Badge variant={PAYMENT_STATUS_BADGE_VARIANT[attempt.status]}>
                          {messages.status[attempt.status]}
                        </Badge>
                      </div>
                      <div className="flex flex-wrap gap-x-4 gap-y-1 text-xs text-[var(--sdk-color-text-secondary)]">
                        <span>{formatPaymentProvider(attempt.providerCode)}</span>
                        <span>{messages.detail.channel}: {attempt.channelId || "--"}</span>
                        <span>{messages.detail.providerTransaction}: {attempt.providerTransactionId || "--"}</span>
                      </div>
                    </div>
                    <div className="text-sm font-semibold">
                      {formatAdminAmount(attempt.amount, attempt.currencyCode)}
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">{messages.detail.noAttempts}</p>
            )}
          </DetailDrawerSection>

          <DetailDrawerSection title={messages.detail.timeline}>
            <ol className="space-y-3">
              <li className="grid grid-cols-[2rem_minmax(0,1fr)] gap-3">
                <span className="flex h-8 w-8 items-center justify-center rounded-full border border-[var(--sdk-color-border-default)] text-[var(--sdk-color-state-success)]">
                  <Clock3 aria-hidden="true" className="h-4 w-4" />
                </span>
                <div className="min-w-0 border-b border-[var(--sdk-color-border-subtle)] pb-3">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <strong className="text-sm">{messages.detail.timelineCreated}</strong>
                    <time className="text-xs text-[var(--sdk-color-text-muted)]">{formatAdminTimestamp(detail.createdAt)}</time>
                  </div>
                  <p className="mt-1 text-sm text-[var(--sdk-color-text-secondary)]">{messages.detail.timelineCreatedDescription}</p>
                </div>
              </li>
              <li className="grid grid-cols-[2rem_minmax(0,1fr)] gap-3">
                <span className="flex h-8 w-8 items-center justify-center rounded-full border border-[var(--sdk-color-border-default)] text-[var(--sdk-color-brand-primary)]">
                  <Clock3 aria-hidden="true" className="h-4 w-4" />
                </span>
                <div className="min-w-0">
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <strong className="text-sm">{messages.detail.timelineUpdated}</strong>
                    <time className="text-xs text-[var(--sdk-color-text-muted)]">{formatAdminTimestamp(detail.updatedAt)}</time>
                  </div>
                  <p className="mt-1 text-sm text-[var(--sdk-color-text-secondary)]">{messages.detail.timelineUpdatedDescription}</p>
                </div>
              </li>
            </ol>
          </DetailDrawerSection>

          <DetailDrawerSection title={messages.detail.metadata}>
            {detail.metadata && Object.keys(detail.metadata).length > 0 ? (
              <pre className="max-h-72 overflow-auto whitespace-pre-wrap break-all bg-[var(--sdk-color-bg-subtle)] p-3 text-xs text-[var(--sdk-color-text-secondary)]">
                {JSON.stringify(detail.metadata, null, 2)}
              </pre>
            ) : (
              <p className="text-sm text-[var(--sdk-color-text-secondary)]">{messages.detail.noMetadata}</p>
            )}
          </DetailDrawerSection>
        </>
      ) : null}
    </DetailDrawer>
  );
}
