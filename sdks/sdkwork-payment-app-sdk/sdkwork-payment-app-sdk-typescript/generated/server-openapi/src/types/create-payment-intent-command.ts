export interface CreatePaymentIntentCommand {
  orderId: string;
  paymentMethod?: string;
}
