export const APP_PAYMENT_METHOD_TREE = {
  payments: {
    methods: { list: true },
    intents: {
      create: true,
      retrieve: true,
      cancel: true,
      attempts: { create: true },
    },
    attempts: { retrieve: true },
    checkout: { retrieve: true },
    close: true,
    create: true,
    orderPayments: { list: true },
    records: {
      list: true,
      retrieve: true,
    },
    reconcile: true,
    statistics: { retrieve: true },
    status: {
      retrieve: true,
      retrieveByOutTradeNo: true,
    },
  },
} as const;

export type PaymentRequestParams = Record<string, unknown>;
export type PaymentSdkResponse<T> = Promise<T>;
export type PaymentSdkMethod = (...args: any[]) => PaymentSdkResponse<any>;

type MethodTree = {
  readonly [key: string]: true | MethodTree;
};

export type ClientFromMethodTree<TTree extends MethodTree> = {
  readonly [TKey in keyof TTree]: TTree[TKey] extends true
    ? PaymentSdkMethod
    : TTree[TKey] extends MethodTree
      ? ClientFromMethodTree<TTree[TKey]>
      : never;
};

export type PaymentAppSdkClient = {
  commerce: ClientFromMethodTree<{ payments: (typeof APP_PAYMENT_METHOD_TREE)["payments"] }>;
};
