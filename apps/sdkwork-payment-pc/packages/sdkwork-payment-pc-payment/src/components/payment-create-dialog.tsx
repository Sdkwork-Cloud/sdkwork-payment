import {
  useEffect,
  useState,
} from "react";
import {
  Button,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
  StatusNotice,
} from "@sdkwork/ui-pc-react";
import type { SdkworkPaymentProductType } from "../payment";
import type { SdkworkPaymentController } from "../payment-controller";
import { useSdkworkPaymentControllerState } from "../payment-controller";
import { useSdkworkPaymentIntl } from "../payment-intl";

export interface SdkworkPaymentCreateDialogProps {
  controller: SdkworkPaymentController;
}

export function SdkworkPaymentCreateDialog({
  controller,
}: SdkworkPaymentCreateDialogProps) {
  const state = useSdkworkPaymentControllerState(controller);
  const {
    copy,
    formatProductType,
  } = useSdkworkPaymentIntl();
  const selectedMethod = state.dashboard.methods.find((method) => method.code === state.selectedMethodCode)
    ?? state.dashboard.methods[0]
    ?? null;
  const [orderId, setOrderId] = useState("");
  const [methodCode, setMethodCode] = useState<string>(selectedMethod?.code ?? "");
  const [productType, setProductType] = useState<SdkworkPaymentProductType>(
    selectedMethod?.recommendedProductType ?? "unknown",
  );

  useEffect(() => {
    if (!state.isCreateOpen) {
      return;
    }

    const nextMethod = selectedMethod?.code ?? "";
    setOrderId("");
    setMethodCode(nextMethod);
    setProductType(selectedMethod?.recommendedProductType ?? "unknown");
  }, [selectedMethod, state.isCreateOpen]);

  const activeMethod = state.dashboard.methods.find((method) => method.code === methodCode) ?? selectedMethod;

  return (
    <Dialog
      onOpenChange={(open) => {
        if (!open) {
          controller.closeCreateDialog();
        }
      }}
      open={state.isCreateOpen}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{copy.createDialog.title}</DialogTitle>
          <DialogDescription>
            {copy.createDialog.description}
          </DialogDescription>
        </DialogHeader>

        {state.lastError ? (
          <StatusNotice title={copy.createDialog.errorTitle} tone="danger">
            {state.lastError}
          </StatusNotice>
        ) : null}

        <form
          className="space-y-4"
          onSubmit={(event) => {
            event.preventDefault();
            void controller.createPayment({
              orderId: orderId.trim(),
              paymentMethod: methodCode,
              productType,
            });
          }}
        >
          <label className="space-y-2 text-sm text-[var(--sdk-color-text-secondary)]">
            <span className="font-medium text-[var(--sdk-color-text-primary)]">{copy.createDialog.orderIdLabel}</span>
            <Input
              onChange={(event) => setOrderId(event.target.value)}
              required
              value={orderId}
            />
          </label>

          <div className="space-y-2 text-sm text-[var(--sdk-color-text-secondary)]">
            <span className="font-medium text-[var(--sdk-color-text-primary)]">{copy.createDialog.paymentMethodLabel}</span>
            <Select
              onValueChange={(value) => {
                setMethodCode(value);
                controller.selectMethod(value);
                const method = state.dashboard.methods.find((item) => item.code === value);
                setProductType(method?.recommendedProductType ?? "unknown");
              }}
              value={methodCode}
            >
              <SelectTrigger aria-label={copy.createDialog.paymentMethodLabel}>
                <SelectValue placeholder={copy.createDialog.paymentMethodPlaceholder} />
              </SelectTrigger>
              <SelectContent>
                {state.dashboard.methods.map((method) => (
                  <SelectItem key={method.code} value={method.code}>
                    {method.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-2 text-sm text-[var(--sdk-color-text-secondary)]">
            <span className="font-medium text-[var(--sdk-color-text-primary)]">{copy.createDialog.productTypeLabel}</span>
            <Select
              onValueChange={(value) => setProductType(value as SdkworkPaymentProductType)}
              value={productType}
            >
              <SelectTrigger aria-label={copy.createDialog.productTypeLabel}>
                <SelectValue placeholder={copy.createDialog.productTypePlaceholder} />
              </SelectTrigger>
              <SelectContent>
                {(activeMethod?.productTypes ?? []).map((item) => (
                  <SelectItem key={`${activeMethod?.code}-${item.code}`} value={item.code}>
                    {item.label || formatProductType(item.code)}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>

          <DialogFooter>
            <Button onClick={() => controller.closeCreateDialog()} type="button" variant="ghost">
              {copy.actions.close}
            </Button>
            <Button disabled={!orderId.trim() || !methodCode} loading={state.isMutating} type="submit">
              {copy.actions.createPayment}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
