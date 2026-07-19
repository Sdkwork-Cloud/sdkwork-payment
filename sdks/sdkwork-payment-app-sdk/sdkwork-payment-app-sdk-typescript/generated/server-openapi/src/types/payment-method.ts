export interface PaymentMethod {
  methodId: string;
  code: string;
  methodName: string;
  available: boolean;
  sort: number;
}
