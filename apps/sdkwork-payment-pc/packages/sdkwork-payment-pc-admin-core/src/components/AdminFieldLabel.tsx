/**
 * Shared admin field label.
 *
 * Wraps a `<Label>` with a consistent layout (label + optional required marker +
 * children) used across all admin capability packages. Extracted to admin-core
 * to eliminate 12+ duplicate definitions.
 */

import * as React from "react";
import { Label } from "@sdkwork/ui-pc-react";

export interface AdminFieldLabelProps {
  children: React.ReactNode;
  htmlFor: string;
  label: string;
  required?: boolean;
}

export function AdminFieldLabel({ children, htmlFor, label, required }: AdminFieldLabelProps) {
  return (
    <div className="space-y-1.5">
      <Label htmlFor={htmlFor}>
        {label}
        {required ? <span className="text-[var(--sdk-color-text-error)]">*</span> : null}
      </Label>
      {children}
    </div>
  );
}
