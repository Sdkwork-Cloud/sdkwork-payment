import {
  useMemo,
  useSyncExternalStore,
} from "react";
import type {
  SdkworkPaymentDetail,
  SdkworkPaymentFilter,
  SdkworkPaymentProductType,
  SdkworkPaymentSummary,
} from "./payment";
import {
  createSdkworkPaymentMessages,
  type SdkworkPaymentMessagesOverrides,
} from "./payment-copy";
import {
  createSdkworkPaymentService,
  type SdkworkPaymentCloseResult,
  type SdkworkPaymentCreateInput,
  type SdkworkPaymentDashboardData,
  type SdkworkPaymentListRecordsResult,
  type SdkworkPaymentPageInfo,
  type SdkworkPaymentService,
} from "./payment-service";

export interface SdkworkPaymentControllerState {
  activeFilter: SdkworkPaymentFilter;
  dashboard: SdkworkPaymentDashboardData;
  detail?: SdkworkPaymentDetail;
  isBootstrapped: boolean;
  isCreateOpen: boolean;
  isDetailLoading: boolean;
  isDetailOpen: boolean;
  isLoading: boolean;
  isMutating: boolean;
  isRecordsLoading: boolean;
  lastError?: string;
  page: number;
  pageSize: number;
  pageInfo?: SdkworkPaymentPageInfo;
  relatedPayments: SdkworkPaymentSummary[];
  selectedMethodCode: string | null;
  selectedPaymentId?: string;
  visibleRecords: SdkworkPaymentSummary[];
}

export interface SdkworkPaymentController {
  bootstrap(): Promise<SdkworkPaymentControllerState>;
  closeCreateDialog(): void;
  closeDetail(): void;
  closePayment(paymentId?: string): Promise<SdkworkPaymentCloseResult>;
  createPayment(
    input: Omit<SdkworkPaymentCreateInput, "paymentMethod" | "productType"> & {
      paymentMethod?: string;
      productType?: SdkworkPaymentProductType;
    },
  ): Promise<SdkworkPaymentDetail>;
  getState(): SdkworkPaymentControllerState;
  loadPage(page: number): Promise<SdkworkPaymentListRecordsResult>;
  openCreateDialog(): void;
  openDetail(paymentId: string): Promise<SdkworkPaymentControllerState>;
  refresh(): Promise<SdkworkPaymentControllerState>;
  refreshPaymentStatus(paymentId?: string): Promise<SdkworkPaymentSummary>;
  reconcilePayment(input?: { orderId?: string; outTradeNo?: string }): Promise<SdkworkPaymentSummary>;
  selectMethod(methodCode: string): void;
  service: SdkworkPaymentService;
  setFilter(filter: SdkworkPaymentFilter): void;
  setPageSize(pageSize: number): Promise<SdkworkPaymentListRecordsResult>;
  subscribe(listener: () => void): () => void;
}

export interface CreateSdkworkPaymentControllerOptions {
  initialState?: Partial<SdkworkPaymentControllerState>;
  locale?: string | null;
  messages?: SdkworkPaymentMessagesOverrides;
  service?: Partial<SdkworkPaymentService>;
}

function deriveVisibleRecords(
  dashboard: SdkworkPaymentDashboardData,
  activeFilter: SdkworkPaymentFilter,
): SdkworkPaymentSummary[] {
  const records = dashboard.records ?? [];

  if (activeFilter === "all") {
    return records;
  }

  if (activeFilter === "actionable") {
    return records.filter(
      (record) => record.status === "default" || record.status === "pending",
    );
  }

  return records.filter((record) => record.status === activeFilter);
}

function resolveSelectedMethodCode(
  dashboard: SdkworkPaymentDashboardData,
  selectedMethodCode: string | null,
): string | null {
  const methods = dashboard.methods ?? [];

  if (selectedMethodCode && methods.some((method) => method.code === selectedMethodCode)) {
    return selectedMethodCode;
  }

  return methods.find((method) => method.available)?.code
    ?? methods[0]?.code
    ?? null;
}

function findSelectedMethod(
  dashboard: SdkworkPaymentDashboardData,
  selectedMethodCode: string | null,
) {
  return dashboard.methods?.find((method) => method.code === selectedMethodCode) ?? null;
}

function mergeDetailWithStatus(
  detail: SdkworkPaymentDetail | undefined,
  status: SdkworkPaymentSummary,
): SdkworkPaymentDetail | undefined {
  if (!detail) {
    return undefined;
  }

  return {
    ...detail,
    ...status,
    canClose: status.canClose,
    canReconcile: status.canReconcile,
    canRefreshStatus: status.canRefreshStatus,
    needQuery: detail.needQuery && status.canRefreshStatus,
  };
}

function normalizeDashboard(
  dashboard: SdkworkPaymentDashboardData | undefined,
  fallbackDashboard: SdkworkPaymentDashboardData,
): SdkworkPaymentDashboardData {
  const resolvedDashboard = dashboard ?? fallbackDashboard;

  return {
    ...fallbackDashboard,
    ...resolvedDashboard,
    digest: resolvedDashboard.digest ?? fallbackDashboard.digest,
    methods: [...(resolvedDashboard.methods ?? fallbackDashboard.methods ?? [])],
    records: [...(resolvedDashboard.records ?? fallbackDashboard.records ?? [])],
    statistics: resolvedDashboard.statistics ?? fallbackDashboard.statistics,
  };
}

function normalizeState(
  state: SdkworkPaymentControllerState,
  fallbackDashboard: SdkworkPaymentDashboardData,
): SdkworkPaymentControllerState {
  const dashboard = normalizeDashboard(state.dashboard, fallbackDashboard);
  const selectedMethodCode = resolveSelectedMethodCode(dashboard, state.selectedMethodCode);

  return {
    ...state,
    dashboard,
    relatedPayments: [...state.relatedPayments],
    selectedMethodCode,
    visibleRecords: deriveVisibleRecords(dashboard, state.activeFilter),
  };
}

function resolvePaymentId(
  state: SdkworkPaymentControllerState,
  paymentId: string | undefined,
  emptySelectionMessage: string,
): string {
  const resolvedPaymentId = paymentId ?? state.selectedPaymentId;
  if (!resolvedPaymentId) {
    throw new Error(emptySelectionMessage);
  }

  return resolvedPaymentId;
}

function resolveReconcileInput(
  state: SdkworkPaymentControllerState,
  input?: { orderId?: string; outTradeNo?: string },
  emptyContextMessage?: string,
): { orderId?: string; outTradeNo?: string } {
  if (input?.orderId || input?.outTradeNo) {
    return input;
  }

  if (state.detail?.orderId) {
    return {
      orderId: state.detail.orderId,
    };
  }

  if (state.detail?.outTradeNo) {
    return {
      outTradeNo: state.detail.outTradeNo,
    };
  }

  throw new Error(emptyContextMessage || "No payment context is available for reconciliation.");
}

export function createSdkworkPaymentController(
  options: CreateSdkworkPaymentControllerOptions = {},
): SdkworkPaymentController {
  const messages = createSdkworkPaymentMessages(options.locale, options.messages);
  const copy = messages.controller;
  const fallbackDashboard = (
    options.service?.getEmptyDashboard
    ?? createSdkworkPaymentService({
      locale: options.locale,
      messages: options.messages,
    }).getEmptyDashboard
  )();
  const service: SdkworkPaymentService = options.service
    ? {
        ...createSdkworkPaymentService({
          locale: options.locale,
          messages: options.messages,
        }),
        ...options.service,
      }
    : createSdkworkPaymentService({
        locale: options.locale,
        messages: options.messages,
      });
  const listeners = new Set<() => void>();
  let state = normalizeState({
    activeFilter: "all",
    dashboard: fallbackDashboard,
    isBootstrapped: false,
    isCreateOpen: false,
    isDetailLoading: false,
    isDetailOpen: false,
    isLoading: false,
    isMutating: false,
    isRecordsLoading: false,
    page: 1,
    pageSize: options.initialState?.pageSize ?? 20,
    relatedPayments: [],
    selectedMethodCode: fallbackDashboard.methods[0]?.code ?? null,
    visibleRecords: fallbackDashboard.records,
    ...options.initialState,
  }, fallbackDashboard);

  function emit(): void {
    listeners.forEach((listener) => listener());
  }

  function setState(
    next:
      | Partial<SdkworkPaymentControllerState>
      | ((currentState: SdkworkPaymentControllerState) => Partial<SdkworkPaymentControllerState>),
  ): void {
    const partial = typeof next === "function" ? next(state) : next;
    state = normalizeState({
      ...state,
      ...partial,
    }, fallbackDashboard);
    emit();
  }

  async function refreshDashboard(options: {
    preserveMethod?: boolean;
  } = {}): Promise<SdkworkPaymentDashboardData> {
    const dashboard = normalizeDashboard(
      await service.getDashboard(),
      state.dashboard,
    );
    setState((currentState) => ({
      dashboard,
      isBootstrapped: true,
      isLoading: false,
      isMutating: false,
      pageInfo: dashboard.pageInfo ?? currentState.pageInfo,
      selectedMethodCode: options.preserveMethod ? currentState.selectedMethodCode : dashboard.methods[0]?.code ?? null,
    }));
    return dashboard;
  }

  async function loadRelatedPayments(orderId: string | undefined): Promise<SdkworkPaymentSummary[]> {
    if (!orderId) {
      return [];
    }

    try {
      return await service.listOrderPayments(orderId);
    } catch {
      return [];
    }
  }

  return {
    async bootstrap() {
      setState({
        isLoading: true,
        lastError: undefined,
      });

      try {
        await refreshDashboard();
        return state;
      } catch (error) {
        setState({
          isLoading: false,
          lastError: error instanceof Error ? error.message : copy.bootstrapFailed,
        });
        throw error;
      }
    },

    closeCreateDialog() {
      setState({
        isCreateOpen: false,
      });
    },

    closeDetail() {
      setState({
        detail: undefined,
        isDetailOpen: false,
        relatedPayments: [],
        selectedPaymentId: undefined,
      });
    },

    async closePayment(paymentId) {
      const resolvedPaymentId = resolvePaymentId(state, paymentId, copy.selectPaymentRequired);
      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const result = await service.closePayment(resolvedPaymentId);
        await refreshDashboard({
          preserveMethod: true,
        });
        if (state.detail?.id === resolvedPaymentId) {
          setState((currentState) => ({
            detail: currentState.detail
              ? {
                  ...currentState.detail,
                  canClose: false,
                  canReconcile: false,
                  canRefreshStatus: false,
                  needQuery: false,
                  status: "closed",
                  statusLabel: messages.status.closed,
                }
              : undefined,
          }));
        }
        return result;
      } catch (error) {
        setState({
          isMutating: false,
          lastError: error instanceof Error ? error.message : copy.closeFailed,
        });
        throw error;
      }
    },

    async createPayment(input) {
      const method = input.paymentMethod
        ?? findSelectedMethod(state.dashboard, state.selectedMethodCode)?.code
        ?? state.selectedMethodCode;
      if (!method) {
        throw new Error(copy.selectPaymentMethodRequired);
      }

      const productType = input.productType
        ?? findSelectedMethod(state.dashboard, method)?.recommendedProductType
        ?? "unknown";

      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const detail = await service.createPayment({
          ...input,
          paymentMethod: method,
          productType,
        });
        const relatedPayments = await loadRelatedPayments(detail.orderId);
        await refreshDashboard({
          preserveMethod: true,
        });
        setState({
          detail,
          isCreateOpen: false,
          isDetailOpen: true,
          relatedPayments,
          selectedMethodCode: method,
          selectedPaymentId: detail.id,
        });
        return detail;
      } catch (error) {
        setState({
          isMutating: false,
          lastError: error instanceof Error ? error.message : copy.createFailed,
        });
        throw error;
      }
    },

    getState() {
      return state;
    },

    openCreateDialog() {
      setState({
        isCreateOpen: true,
        lastError: undefined,
      });
    },

    async openDetail(paymentId) {
      setState({
        isDetailLoading: true,
        isDetailOpen: true,
        lastError: undefined,
        selectedPaymentId: paymentId,
      });

      try {
        const detail = await service.getPaymentDetail(paymentId);
        const relatedPayments = await loadRelatedPayments(detail.orderId);
        setState({
          detail,
          isDetailLoading: false,
          isDetailOpen: true,
          relatedPayments,
          selectedPaymentId: paymentId,
        });
        return state;
      } catch (error) {
        setState({
          isDetailLoading: false,
          lastError: error instanceof Error ? error.message : copy.detailFailed,
        });
        throw error;
      }
    },

    async refresh() {
      await refreshDashboard({
        preserveMethod: true,
      });
      return state;
    },

    async loadPage(page) {
      const pageNumber = Math.max(1, Math.min(page, state.pageInfo?.totalPages ?? 1));
      setState({
        isRecordsLoading: true,
        lastError: undefined,
        page: pageNumber,
      });

      try {
        const result = await service.listRecords({
          page: pageNumber,
          pageSize: state.pageSize,
        });
        setState((currentState) => ({
          dashboard: {
            ...currentState.dashboard,
            records: result.records,
          },
          isRecordsLoading: false,
          pageInfo: result.pageInfo,
          visibleRecords: deriveVisibleRecords(
            {
              ...currentState.dashboard,
              records: result.records,
            },
            currentState.activeFilter,
          ),
        }));
        return result;
      } catch (error) {
        setState({
          isRecordsLoading: false,
          lastError: error instanceof Error ? error.message : copy.bootstrapFailed,
        });
        throw error;
      }
    },

    async setPageSize(pageSize) {
      const clampedPageSize = Math.min(200, Math.max(1, pageSize));
      setState({
        isRecordsLoading: true,
        lastError: undefined,
        page: 1,
        pageSize: clampedPageSize,
      });

      try {
        const result = await service.listRecords({
          page: 1,
          pageSize: clampedPageSize,
        });
        setState((currentState) => ({
          dashboard: {
            ...currentState.dashboard,
            records: result.records,
          },
          isRecordsLoading: false,
          pageInfo: result.pageInfo,
          visibleRecords: deriveVisibleRecords(
            {
              ...currentState.dashboard,
              records: result.records,
            },
            currentState.activeFilter,
          ),
        }));
        return result;
      } catch (error) {
        setState({
          isRecordsLoading: false,
          lastError: error instanceof Error ? error.message : copy.bootstrapFailed,
        });
        throw error;
      }
    },

    async refreshPaymentStatus(paymentId) {
      const resolvedPaymentId = resolvePaymentId(state, paymentId, copy.selectPaymentRequired);
      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const status = await service.getPaymentStatus(resolvedPaymentId);
        const relatedPayments = await loadRelatedPayments(status.orderId ?? state.detail?.orderId);
        await refreshDashboard({
          preserveMethod: true,
        });
        setState((currentState) => ({
          detail: currentState.selectedPaymentId === resolvedPaymentId
            ? mergeDetailWithStatus(currentState.detail, status)
            : currentState.detail,
          relatedPayments,
        }));
        return status;
      } catch (error) {
        setState({
          isMutating: false,
          lastError: error instanceof Error ? error.message : copy.statusFailed,
        });
        throw error;
      }
    },

    async reconcilePayment(input) {
      const payload = resolveReconcileInput(state, input, copy.reconcileContextRequired);
      setState({
        isMutating: true,
        lastError: undefined,
      });

      try {
        const status = await service.reconcilePayment(payload);
        const relatedPayments = await loadRelatedPayments(status.orderId ?? state.detail?.orderId);
        await refreshDashboard({
          preserveMethod: true,
        });
        setState((currentState) => ({
          detail: mergeDetailWithStatus(currentState.detail, status),
          relatedPayments,
        }));
        return status;
      } catch (error) {
        setState({
          isMutating: false,
          lastError: error instanceof Error ? error.message : copy.reconcileFailed,
        });
        throw error;
      }
    },

    selectMethod(methodCode) {
      setState({
        selectedMethodCode: methodCode,
      });
    },

    service,

    setFilter(filter) {
      setState({
        activeFilter: filter,
      });
    },

    subscribe(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
      };
    },
  };
}

export function useSdkworkPaymentController(
  controller?: SdkworkPaymentController,
  options?: Pick<CreateSdkworkPaymentControllerOptions, "locale" | "messages" | "service">,
): SdkworkPaymentController {
  return useMemo(
    () => controller ?? createSdkworkPaymentController({
      ...(options?.locale ? { locale: options.locale } : {}),
      ...(options?.messages ? { messages: options.messages } : {}),
      ...(options?.service ? { service: options.service } : {}),
    }),
    [controller, options?.locale, options?.messages, options?.service],
  );
}

export function useSdkworkPaymentControllerState(
  controller: SdkworkPaymentController,
): SdkworkPaymentControllerState {
  return useSyncExternalStore(
    controller.subscribe,
    controller.getState,
    controller.getState,
  );
}
