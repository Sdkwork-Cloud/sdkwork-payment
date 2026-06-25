import { describe, expect, it } from "vitest";
import {
  createSdkworkPaymentBackdropStyle,
  createSdkworkPaymentHeroStyle,
  createSdkworkPaymentPanelStyle,
  createSdkworkPaymentQrSurfaceStyle,
  createSdkworkPaymentToneStyle,
} from "../src";

describe("sdkwork-payment-pc-payment appearance", () => {
  it("exports Sdkwork-style payment backdrop, hero, panel, and tone helpers", () => {
    expect(createSdkworkPaymentToneStyle("brand").color).toBe("var(--sdk-color-brand-primary)");
    expect(createSdkworkPaymentPanelStyle("accent").backgroundImage).toContain("var(--sdk-color-brand-accent)");
    expect(createSdkworkPaymentPanelStyle("accent").backgroundImage).toContain("var(--sdk-color-surface-panel)");
    expect(createSdkworkPaymentHeroStyle().backgroundImage).toContain("var(--sdk-color-brand-accent)");
    expect(createSdkworkPaymentHeroStyle().backgroundImage).toContain("var(--sdk-color-surface-canvas)");
    expect(createSdkworkPaymentHeroStyle().backgroundImage).toContain("var(--sdk-color-surface-panel)");
    expect(createSdkworkPaymentHeroStyle().backgroundImage).not.toContain("#18181b");
    expect(createSdkworkPaymentBackdropStyle().backgroundImage).toContain("var(--sdk-color-brand-primary)");
    expect(createSdkworkPaymentQrSurfaceStyle().backgroundColor).toContain("var(--sdk-color-surface-canvas)");
    expect(createSdkworkPaymentQrSurfaceStyle().boxShadow).toBe("var(--sdk-shadow-sm)");
  });
});
