import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { PaymentAppShell } from "@sdkwork/payment-pc-shell";
import { ErrorBoundary } from "./ErrorBoundary";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ErrorBoundary>
      <PaymentAppShell />
    </ErrorBoundary>
  </StrictMode>,
);
