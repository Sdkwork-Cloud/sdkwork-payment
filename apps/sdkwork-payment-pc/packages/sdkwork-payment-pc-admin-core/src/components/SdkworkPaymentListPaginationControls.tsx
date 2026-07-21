import type { SdkWorkPageInfo } from "@sdkwork/payment-contracts";
import { Button } from "@sdkwork/ui-pc-react";
import { ChevronDown, LoaderCircle } from "lucide-react";

export interface SdkworkPaymentListPaginationControlsProps {
  busy?: boolean;
  label?: string;
  onLoadMore?: () => void | Promise<void>;
  pageInfo?: SdkWorkPageInfo;
  summary?: string;
}

export function SdkworkPaymentListPaginationControls({
  busy = false,
  label = "Load more",
  onLoadMore,
  pageInfo,
  summary,
}: SdkworkPaymentListPaginationControlsProps) {
  const totalItems = pageInfo?.totalItems ? Number(pageInfo.totalItems) : undefined;
  const loadedCount = pageInfo?.page && pageInfo.pageSize ? pageInfo.page * pageInfo.pageSize : undefined;
  const defaultSummary =
    totalItems !== undefined && Number.isFinite(totalItems)
      ? `Showing ${Math.min(loadedCount ?? totalItems, totalItems)} of ${totalItems}`
      : undefined;
  const resolvedSummary = summary ?? defaultSummary;
  const canLoadMore = Boolean(pageInfo?.hasMore && onLoadMore);

  if (!resolvedSummary && !canLoadMore) {
    return null;
  }

  return (
    <div
      className="flex min-w-0 flex-1 flex-col gap-3 py-0.5 sm:flex-row sm:items-center sm:justify-between"
      data-sdk-region="payment-list-pagination"
    >
      {resolvedSummary ? (
        <span className="min-w-0 text-sm tabular-nums text-[var(--sdk-color-text-secondary)]">
          {resolvedSummary}
        </span>
      ) : null}
      {canLoadMore ? (
        <Button
          aria-busy={busy}
          className="w-full shrink-0 sm:w-auto"
          disabled={busy}
          onClick={() => void onLoadMore?.()}
          size="sm"
          type="button"
          variant="outline"
        >
          {busy ? (
            <LoaderCircle aria-hidden="true" className="mr-2 h-4 w-4 animate-spin" />
          ) : (
            <ChevronDown aria-hidden="true" className="mr-2 h-4 w-4" />
          )}
          {label}
        </Button>
      ) : null}
    </div>
  );
}
