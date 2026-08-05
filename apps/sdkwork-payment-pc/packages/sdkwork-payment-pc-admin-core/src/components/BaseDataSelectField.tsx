/**
 * Base-data select field shared across payment admin forms.
 *
 * Country/currency pickers backed by options resolved from the
 * sdkwork-appbase base-data capability (base_country / base_currency).
 * The host app supplies `options`; when no options are available the field
 * degrades to the previous free-text input, so forms stay functional during
 * a base-data service outage. A persisted legacy value that is missing from
 * the option list stays selectable when editing existing records.
 */

import * as React from "react";
import { Input, Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@sdkwork/ui-pc-react";
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
  if (options.length === 0) {
    return (
      <AdminFieldLabel label={props.label} htmlFor={props.id} required={props.required}>
        <Input
          id={props.id}
          value={props.value}
          onChange={(event) => props.onChange(event.target.value)}
          maxLength={props.maxLength}
          placeholder={props.placeholder}
          disabled={props.disabled}
        />
      </AdminFieldLabel>
    );
  }
  return (
    <AdminFieldLabel label={props.label} htmlFor={props.id} required={props.required}>
      <Select value={props.value} onValueChange={props.onChange} disabled={props.disabled}>
        <SelectTrigger id={props.id}>
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {includeCurrentOption(options, props.value).map((option) => (
            <SelectItem key={option.value} value={option.value}>
              {option.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
    </AdminFieldLabel>
  );
}

/** Keeps a persisted legacy value selectable when editing existing records. */
function includeCurrentOption(
  options: readonly PaymentBaseDataOption[],
  currentValue: string,
): readonly PaymentBaseDataOption[] {
  if (!currentValue || options.some((option) => option.value === currentValue)) {
    return options;
  }
  return [...options, { value: currentValue, label: `${currentValue} (legacy)` }];
}
