export interface CreatePaymentCommand {
  orderId: string;
  paymentMethod?: string;
  amount?: string;
  businessOrderId?: string;
  businessType?: string;
  clientIp?: string;
  /** WeChat JSAPI payer openid */
  payerOpenId?: string;
  paymentProvider?: string;
  paymentScene?: string;
  productType?: string;
}
