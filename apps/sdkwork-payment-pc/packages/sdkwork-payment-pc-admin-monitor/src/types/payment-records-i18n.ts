import type {
  PaymentStatus,
  ReconciliationRunStatus,
  ReconciliationType,
  RefundReasonCode,
  RefundStatus,
  WebhookEventStatus,
  WebhookSignatureStatus,
} from "./monitor-admin-types";

export interface PaymentRecordsMessages {
  actions: {
    applyFilters: string;
    clearFilters: string;
    close: string;
    copyIdentifier: string;
    hideAdvanced: string;
    loadMore: string;
    refresh: string;
    showAdvanced: string;
    viewDetails: string;
  };
  detail: {
    amount: string;
    attempts: string;
    attemptsDescription: string;
    channel: string;
    createdAt: string;
    customerAndOrder: string;
    intentIdentifier: string;
    metadata: string;
    method: string;
    noAttempts: string;
    noMetadata: string;
    orderIdentifier: string;
    ownerIdentifier: string;
    paymentRecord: string;
    provider: string;
    providerTransaction: string;
    status: string;
    timeline: string;
    timelineCreated: string;
    timelineCreatedDescription: string;
    timelineUpdated: string;
    timelineUpdatedDescription: string;
    updatedAt: string;
  };
  empty: {
    description: string;
    title: string;
  };
  filters: {
    activeSummary: (count: number) => string;
    advancedDescription: string;
    allProviders: string;
    allStatuses: string;
    createdFrom: string;
    createdTo: string;
    currency: string;
    currencyPlaceholder: string;
    description: string;
    last30Days: string;
    last7Days: string;
    orderIdentifier: string;
    orderPlaceholder: string;
    ownerIdentifier: string;
    ownerPlaceholder: string;
    provider: string;
    search: string;
    searchPlaceholder: string;
    status: string;
    title: string;
    today: string;
  };
  metrics: {
    currentResultSet: string;
    exceptionDescription: string;
    exceptionLabel: string;
    loadedDescription: (count: number) => string;
    multipleCurrencies: (count: number) => string;
    recordsLabel: string;
    successfulVolumeDescription: string;
    successfulVolumeLabel: string;
    successRateDescription: string;
    successRateLabel: string;
  };
  operations: {
    actions: {
      applyFilter: string;
      cancel: string;
      clearFilters: string;
      createReconciliation: string;
      createRun: string;
      createRefund: string;
      loadMore: string;
      newReconciliation: string;
      newRefund: string;
      replay: string;
      retryRefund: string;
      view: string;
    };
    availability: {
      applyFilter: string;
      cancelCreate: string;
      createReconciliation: string;
      createRefund: string;
      creating: string;
      openDetails: string;
      replay: string;
      replayDisabled: string;
      retryRefund: string;
    };
    attempts: {
      empty: string;
      fields: {
        amount: string;
        channel: string;
        created: string;
        intent: string;
        outTradeNumber: string;
        paidAt: string;
        providerTransaction: string;
      };
      intentIdentifier: string;
      intentPlaceholder: string;
      status: Record<PaymentStatus, string>;
    };
    filters: {
      allProviders: string;
      allStatuses: string;
      provider: string;
      search: string;
      searchPlaceholder: string;
      status: string;
    };
    reconciliation: {
      empty: string;
      fields: {
        account: string;
        created: string;
        difference: string;
        matched: string;
        mismatched: string;
        period: string;
        unmatched: string;
      };
      form: {
        currency: string;
        last7Days: string;
        last7DaysDescription: string;
        lastMonth: string;
        lastMonthDescription: string;
        periodEnd: string;
        periodStart: string;
        provider: string;
        providerAccount: string;
        quickPresets: string;
        reconciliationType: string;
        selectAccount: string;
        yesterday: string;
        yesterdayDescription: string;
      };
      providerAccount: string;
      providerAccountPlaceholder: string;
      status: Record<ReconciliationRunStatus, string>;
      type: Record<ReconciliationType, string>;
    };
    refunds: {
      availability: {
        viewDetails: string;
      };
      confirmationDescription: (refundNo: string) => string;
      confirmationTitle: string;
      createDescription: string;
      detailDescription: (reason: string) => string;
      empty: string;
      emptyFiltered: string;
      fields: {
        actions: string;
        amount: string;
        created: string;
        order: string;
        payment: string;
        paymentAttempt: string;
        providerAccount: string;
        reason: string;
        requestedBy: string;
        status: string;
        updated: string;
      };
      form: {
        amount: string;
        amountHint: string;
        confirmation: string;
        confirmationHint: string;
        noEligiblePayments: string;
        payment: string;
        paymentPlaceholder: string;
        reason: string;
        retryConfirmation: string;
      };
      reason: Record<RefundReasonCode, string>;
      status: Record<RefundStatus, string>;
      summary: {
        completed: string;
        inFlight: string;
        label: string;
        loaded: string;
        needsAttention: string;
      };
      tableDescription: (count: number) => string;
    };
    validation: {
      applyFilterFailed: string;
      clearFiltersFailed: string;
      createReconciliationFailed: string;
      createRefundFailed: string;
      periodInvalid: string;
      periodOrder: string;
      periodRequired: string;
      providerAccountRequired: string;
      replayFailed: string;
      refundAmountExceedsPayment: string;
      refundAmountInvalid: string;
      refundConfirmationRequired: string;
      refundPaymentRequired: string;
      retryRefundFailed: string;
    };
    webhooks: {
      confirmationDescription: (eventType: string, eventId: string) => string;
      confirmationTitle: string;
      detailDescription: (eventId: string, provider: string) => string;
      empty: string;
      eventType: string;
      eventTypePlaceholder: string;
      fields: {
        eventIdentifier: string;
        headers: string;
        lastError: string;
        payload: string;
        processed: string;
        provider: string;
        received: string;
        retries: string;
        signature: string;
        status: string;
      };
      noPayload: string;
      replayResult: (accepted: boolean, eventId: string, replayedAt: string) => string;
      signature: Record<WebhookSignatureStatus, string>;
      status: Record<WebhookEventStatus, string>;
    };
  };
  status: Record<PaymentStatus, string>;
  table: {
    amount: string;
    createdAt: string;
    loading: string;
    paginationSummary: (loaded: number, total: string) => string;
    payment: string;
    providerAndMethod: string;
    references: string;
    resultDescription: (count: number) => string;
    status: string;
    title: string;
  };
  validation: {
    invalidDateRange: string;
    loadDetailFailed: string;
    refreshFailed: string;
  };
  workspace: {
    description: string;
    tabsLabel: string;
    tabs: {
      attempts: string;
      paymentRecords: string;
      reconciliation: string;
      refunds: string;
      webhooks: string;
    };
    title: string;
  };
}
