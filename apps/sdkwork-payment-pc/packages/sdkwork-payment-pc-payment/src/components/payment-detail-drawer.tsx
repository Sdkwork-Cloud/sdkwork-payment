import {
  useEffect,
  useRef,
  useState,
} from "react";
import * as QRCode from "qrcode";
import {
  getSdkworkMediaDeliveryUrl,
} from "@sdkwork/payment-service";
import {
  Button,
  DetailDrawer,
  DetailDrawerMetric,
  DetailDrawerMetrics,
  DetailDrawerSection,
} from "@sdkwork/ui-pc-react";
import type { SdkworkPaymentController } from "../payment-controller";
import { useSdkworkPaymentControllerState } from "../payment-controller";
import {
  createSdkworkPaymentQrSurfaceStyle,
  resolveSdkworkPaymentStatusTone,
} from "../payment-appearance";
import { useSdkworkPaymentIntl } from "../payment-intl";

export interface SdkworkPaymentDetailDrawerProps {
  controller: SdkworkPaymentController;
}

const PAYMENT_STATUS_POLL_INTERVAL_MS = 3000;
const PAYMENT_STATUS_POLL_MAX_ROUNDS = 60;

export function SdkworkPaymentDetailDrawer({
  controller,
}: SdkworkPaymentDetailDrawerProps) {
  const state = useSdkworkPaymentControllerState(controller);
  const {
    copy,
    formatCurrencyCny,
    formatPaymentSummary,
    formatPollingText,
    formatProductType,
    formatStatus,
    formatTimestamp,
  } = useSdkworkPaymentIntl();
  const detail = state.detail;
  const [qrImageSrc, setQrImageSrc] = useState<string>();
  const pollRoundRef = useRef(0);

  useEffect(() => {
    let cancelled = false;

    async function renderQrCode(): Promise<void> {
      if (!detail?.qrImage && !detail?.qrContent) {
        setQrImageSrc(undefined);
        return;
      }

      const qrImageResourceSrc = getSdkworkMediaDeliveryUrl(detail.qrImage);
      if (qrImageResourceSrc) {
        setQrImageSrc(qrImageResourceSrc);
        return;
      }

      try {
        const nextQrImageSrc = await QRCode.toDataURL(detail.qrContent || "", {
          margin: 0,
          width: 240,
        });
        if (!cancelled) {
          setQrImageSrc(nextQrImageSrc);
        }
      } catch {
        if (!cancelled) {
          setQrImageSrc(undefined);
        }
      }
    }

    void renderQrCode();

    return () => {
      cancelled = true;
    };
  }, [detail?.qrContent, detail?.qrImage]);

  useEffect(() => {
    pollRoundRef.current = 0;
  }, [detail?.id]);

  useEffect(() => {
    if (!state.isDetailOpen || !detail) {
      return;
    }
    // 仅在 needQuery=true 且状态可刷新（default/pending）时启动轮询；
    // 状态为 success/failed/timeout/closed 时停止，避免无谓的请求与 OOM 风险。
    if (!detail.needQuery || !detail.canRefreshStatus) {
      return;
    }

    const intervalSeconds = detail.queryIntervalSeconds && detail.queryIntervalSeconds > 0
      ? detail.queryIntervalSeconds
      : PAYMENT_STATUS_POLL_INTERVAL_MS / 1000;
    const intervalMs = Math.max(1000, Math.round(intervalSeconds * 1000));

    let active = true;

    const poll = async (): Promise<void> => {
      if (!active) {
        return;
      }
      if (pollRoundRef.current >= PAYMENT_STATUS_POLL_MAX_ROUNDS) {
        active = false;
        return;
      }
      pollRoundRef.current += 1;
      try {
        const status = await controller.refreshPaymentStatus(detail.id);
        if (!active) {
          return;
        }
        // 终止条件：状态离开 default/pending
        if (status.status !== "default" && status.status !== "pending") {
          active = false;
        }
      } catch {
        // 单次轮询失败不中断整体轮询；下一轮仍然会按节流间隔触发。
      }
    };

    const timer = window.setInterval(poll, intervalMs);
    return () => {
      active = false;
      window.clearInterval(timer);
    };
  }, [
    controller,
    detail?.id,
    detail?.needQuery,
    detail?.canRefreshStatus,
    detail?.queryIntervalSeconds,
    state.isDetailOpen,
  ]);

  return (
    <DetailDrawer
      description={detail?.subject || copy.detail.descriptionFallback}
      footer={(
        <div className="flex flex-wrap justify-end gap-3">
          {detail?.canRefreshStatus ? (
            <Button onClick={() => void controller.refreshPaymentStatus(detail.id)} type="button" variant="outline">
              {copy.actions.refreshStatus}
            </Button>
          ) : null}
          {detail?.canReconcile ? (
            <Button onClick={() => void controller.reconcilePayment()} type="button" variant="outline">
              {copy.actions.reconcile}
            </Button>
          ) : null}
          {detail?.canClose ? (
            <Button onClick={() => void controller.closePayment(detail.id)} type="button" variant="outline">
              {copy.actions.closePayment}
            </Button>
          ) : null}
          <Button onClick={() => controller.closeDetail()} type="button" variant="ghost">
            {copy.actions.close}
          </Button>
        </div>
      )}
      onOpenChange={(open) => {
        if (!open) {
          controller.closeDetail();
        }
      }}
      open={state.isDetailOpen}
      summary={detail ? formatPaymentSummary(detail.id) : copy.detail.summaryLoading}
      title={copy.detail.title}
    >
      {state.isDetailLoading || !detail ? (
        <div className="text-sm text-[var(--sdk-color-text-secondary)]">{copy.detail.loading}</div>
      ) : (
        <>
          <DetailDrawerMetrics columns={3}>
            <DetailDrawerMetric label={copy.detail.amountMetricLabel} value={formatCurrencyCny(detail.amountCny)} />
            <DetailDrawerMetric
              label={copy.detail.statusMetricLabel}
              tone={resolveSdkworkPaymentStatusTone(detail.status)}
              value={formatStatus(detail.status)}
            />
            <DetailDrawerMetric
              label={copy.detail.methodMetricLabel}
              value={detail.paymentMethod || copy.common.emptyValue}
            />
          </DetailDrawerMetrics>

          <DetailDrawerSection description={copy.detail.overviewDescription} title={copy.detail.overviewTitle}>
            <div className="grid gap-3 text-sm text-[var(--sdk-color-text-secondary)] sm:grid-cols-2">
              <div>{copy.detail.paymentIdLabel}: {detail.id}</div>
              <div>{copy.detail.paymentSerialLabel}: {detail.paymentSn || copy.common.emptyValue}</div>
              <div>{copy.detail.orderIdLabel}: {detail.orderId || copy.common.emptyValue}</div>
              <div>{copy.detail.outTradeNoLabel}: {detail.outTradeNo || copy.common.emptyValue}</div>
              <div>{copy.detail.providerLabel}: {detail.paymentProvider || copy.common.emptyValue}</div>
              <div>{copy.detail.transactionIdLabel}: {detail.transactionId || copy.common.emptyValue}</div>
              <div>{copy.detail.productTypeLabel}: {formatProductType(detail.productType)}</div>
              <div>{copy.detail.createdAtLabel}: {formatTimestamp(detail.createdAt)}</div>
              <div>{copy.detail.successTimeLabel}: {formatTimestamp(detail.successTime)}</div>
            </div>
          </DetailDrawerSection>

          {(qrImageSrc || detail.paymentUrl || detail.qrContent) ? (
            <DetailDrawerSection description={copy.detail.scanDescription} title={copy.detail.scanTitle}>
              <div className="grid gap-4 lg:grid-cols-[minmax(0,18rem)_minmax(0,1fr)]">
                <div className="rounded-[1.5rem] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] p-4">
                  {qrImageSrc ? (
                    <img
                      alt={copy.detail.qrImageAlt}
                      className="mx-auto h-60 w-60 rounded-[1rem] p-3"
                      src={qrImageSrc}
                      style={createSdkworkPaymentQrSurfaceStyle()}
                    />
                  ) : (
                    <div className="flex h-60 items-center justify-center rounded-[1rem] border border-dashed border-[var(--sdk-color-border-default)] text-sm text-[var(--sdk-color-text-secondary)]">
                      {copy.detail.qrUnavailable}
                    </div>
                  )}
                </div>

                <div className="space-y-3 text-sm text-[var(--sdk-color-text-secondary)]">
                  <div>
                    <div className="font-medium text-[var(--sdk-color-text-primary)]">{copy.detail.paymentLinkLabel}</div>
                    {detail.paymentUrl ? (
                      <a
                        className="mt-1 inline-flex text-[var(--sdk-color-brand-primary)] underline underline-offset-4"
                        href={detail.paymentUrl}
                        rel="noreferrer"
                        target="_blank"
                      >
                        {detail.paymentUrl}
                      </a>
                    ) : (
                      <div className="mt-1">{copy.common.emptyValue}</div>
                    )}
                  </div>

                  <div>
                    <div className="font-medium text-[var(--sdk-color-text-primary)]">{copy.detail.qrPayloadLabel}</div>
                    <div className="mt-1 break-all">{detail.qrContent || copy.common.emptyValue}</div>
                  </div>

                  <div>
                    <div className="font-medium text-[var(--sdk-color-text-primary)]">{copy.detail.pollingLabel}</div>
                    <div className="mt-1">{formatPollingText(detail.queryIntervalSeconds, detail.needQuery)}</div>
                  </div>
                </div>
              </div>
            </DetailDrawerSection>
          ) : null}

          <DetailDrawerSection description={copy.detail.historyDescription} title={copy.detail.historyTitle}>
            <div className="space-y-3">
              {state.relatedPaymentsError ? (
                <div
                  role="alert"
                  className="rounded-md border border-[var(--sdk-color-border-error)] bg-[var(--sdk-color-bg-error-subtle)] p-3 text-sm text-[var(--sdk-color-text-error)]"
                >
                  {state.relatedPaymentsError}
                </div>
              ) : state.relatedPayments.length === 0 ? (
                <div className="text-sm text-[var(--sdk-color-text-secondary)]">
                  {copy.empty.relatedPayments}
                </div>
              ) : state.relatedPayments.map((payment) => (
                <div
                  className="rounded-[1rem] border border-[var(--sdk-color-border-default)] bg-[var(--sdk-color-surface-panel-muted)] px-4 py-3"
                  key={payment.id}
                >
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div>
                      <div className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                        {payment.paymentMethod || payment.paymentProvider || copy.common.paymentAttempt}
                      </div>
                      <div className="mt-1 text-xs text-[var(--sdk-color-text-secondary)]">
                        {formatTimestamp(payment.createdAt)}
                      </div>
                    </div>
                    <div className="text-right">
                      <div className="text-sm font-semibold text-[var(--sdk-color-text-primary)]">
                        {formatCurrencyCny(payment.amountCny)}
                      </div>
                      <div className="mt-1 text-xs uppercase tracking-[0.14em] text-[var(--sdk-color-text-muted)]">
                        {formatStatus(payment.status)}
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          </DetailDrawerSection>
        </>
      )}
    </DetailDrawer>
  );
}
