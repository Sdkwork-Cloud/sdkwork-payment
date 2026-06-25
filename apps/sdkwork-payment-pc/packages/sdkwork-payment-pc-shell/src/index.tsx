import { sdkworkPaymentPcRuntimeIdentity } from "@sdkwork/payment-pc-core";

export function PaymentAppShell() {
  return (
    <main className="payment-shell">
      <section className="payment-card">
        <h1>SDKWork Payment</h1>
        <p>{sdkworkPaymentPcRuntimeIdentity.appKey}</p>
        <p>Payment capability PC surface — aligned with sdkwork-specs building-block model.</p>
      </section>
    </main>
  );
}
