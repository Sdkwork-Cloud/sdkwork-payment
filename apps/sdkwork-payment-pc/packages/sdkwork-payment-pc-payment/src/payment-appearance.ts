import type { CSSProperties } from "react";
import { createSdkworkBackdropStyle, createSdkworkGlassStyle, createSdkworkHeroStyle, createSdkworkPanelStyle, createSdkworkToneStyle, type SdkworkThemeVisualTone } from "@sdkwork/ui-pc-react/theme";

export type SdkworkPaymentVisualTone = SdkworkThemeVisualTone;

export type SdkworkPaymentMetricTone = "default" | "danger" | "success" | "warning";

export function resolveSdkworkPaymentStatusTone(
  status: string | null | undefined,
): SdkworkPaymentMetricTone {
  const normalized = String(status || "").trim().toLowerCase();

  if (normalized === "success") {
    return "success";
  }

  if (normalized === "pending" || normalized === "default") {
    return "warning";
  }

  if (normalized === "failed" || normalized === "timeout" || normalized === "closed") {
    return "danger";
  }

  return "default";
}

export function createSdkworkPaymentToneStyle(
  tone: SdkworkPaymentVisualTone,
  options: {
    backgroundWeight?: number;
    borderWeight?: number;
  } = {},
): CSSProperties {
  return createSdkworkToneStyle(tone, options);
}

export function createSdkworkPaymentPanelStyle(
  tone: SdkworkPaymentVisualTone,
  options: {
    backgroundWeight?: number;
    borderWeight?: number;
    surfaceColor?: string;
    surfaceWeight?: number;
  } = {},
): CSSProperties {
  return createSdkworkPanelStyle(tone, options);
}

export function createSdkworkPaymentGlassStyle(
  tone: SdkworkPaymentVisualTone,
  options: {
    backgroundWeight?: number;
    borderWeight?: number;
    surfaceColor?: string;
    surfaceWeight?: number;
  } = {},
): CSSProperties {
  return createSdkworkGlassStyle(tone, options);
}

export function createSdkworkPaymentBackdropStyle(): CSSProperties {
  return createSdkworkBackdropStyle();
}

export function createSdkworkPaymentHeroStyle(): CSSProperties {
  return createSdkworkHeroStyle();
}

export function createSdkworkPaymentHeroTextStyle(
  tone: "muted" | "primary" | "subtle" = "primary",
): CSSProperties {
  if (tone === "muted") {
    return {
      color: "color-mix(in srgb, white 72%, var(--sdk-color-brand-accent))",
    };
  }

  if (tone === "subtle") {
    return {
      color: "color-mix(in srgb, white 64%, var(--sdk-color-brand-accent))",
    };
  }

  return {
    color: "color-mix(in srgb, white 92%, var(--sdk-color-brand-accent))",
  };
}

export function createSdkworkPaymentQrSurfaceStyle(): CSSProperties {
  return {
    backgroundColor: "color-mix(in srgb, var(--sdk-color-surface-canvas) 94%, white)",
    boxShadow: "var(--sdk-shadow-sm)",
  };
}

export function createSdkworkPaymentMetricToneStyle(
  tone: SdkworkPaymentMetricTone,
): CSSProperties {
  if (tone === "success") {
    return createSdkworkPaymentToneStyle("success", {
      backgroundWeight: 14,
      borderWeight: 26,
    });
  }

  if (tone === "warning") {
    return createSdkworkPaymentToneStyle("warning", {
      backgroundWeight: 14,
      borderWeight: 26,
    });
  }

  if (tone === "danger") {
    return createSdkworkPaymentToneStyle("danger", {
      backgroundWeight: 14,
      borderWeight: 26,
    });
  }

  return createSdkworkPaymentToneStyle("neutral", {
    backgroundWeight: 10,
    borderWeight: 22,
  });
}
