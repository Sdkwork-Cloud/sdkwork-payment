/**
 * Base-data select field shared across payment admin forms.
 *
 * Country/currency pickers backed by options resolved from the
 * sdkwork-appbase base-data capability (base_country / base_currency).
 * The host app supplies `options`; when no options are available the field
 * degrades to the previous free-text input, so forms stay functional during
 * a base-data service outage. A persisted legacy value that is missing from
 * the option list stays selectable when editing existing records.
 *
 * Rendered through the shared `SdkworkSearchableSelect` (sdkwork-appbase)
 * so the field supports typing to filter codes and names.
 */

import * as React from "react";
import { SdkworkSearchableSelect } from "@sdkwork/appbase-pc-react";
import { AdminFieldLabel } from "./AdminFieldLabel";

/** Single base-data select option (countries/currencies). */
export interface PaymentBaseDataOption {
  readonly value: string;
  readonly label: string;
}

export interface BaseDataSelectFieldProps {
  id: string;
  label: string;
  value: string;
  options?: readonly PaymentBaseDataOption[];
  onChange(value: string): void;
  maxLength?: number;
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
}

export function BaseDataSelectField(props: BaseDataSelectFieldProps) {
  const options = props.options ?? [];
  return (
    <AdminFieldLabel label={props.label} htmlFor={props.id} required={props.required}>
      <SdkworkSearchableSelect
        disabled={props.disabled}
        id={props.id}
        maxLength={props.maxLength}
        onValueChange={props.onChange}
        options={options.length > 0 ? options : null}
        placeholder={props.placeholder}
        value={props.value}
      />
    </AdminFieldLabel>
  );
}
