import type { SdkworkPaymentPcRouteContribution } from "@sdkwork/payment-pc-core";

export const sdkworkPaymentPcPaymentRoutes = [
  {
    auth: "required",
    capability: "payment",
    domain: "commerce",
    id: "app.commerce.payment.dashboard",
    packageName: "@sdkwork/payment-pc-payment",
    path: "/app/payment",
    screen: "dashboard",
    surface: "app",
    title: "Payment",
    titleKey: "payment.routes.dashboard.title",
  },
] as const satisfies readonly SdkworkPaymentPcRouteContribution[];
