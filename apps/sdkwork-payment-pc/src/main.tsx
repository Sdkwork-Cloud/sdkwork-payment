import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { PaymentAppShell } from "@sdkwork/payment-pc-shell";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <PaymentAppShell />
  </StrictMode>,
);
