import { assertSdkworkCatalogLocaleParity } from "@sdkwork/i18n-pc-react";
import { describe, expect, it } from "vitest";

import { PAYMENT_ADMIN_I18N_CATALOG } from "../src/i18n";

describe("PAYMENT_ADMIN_I18N_CATALOG", () => {
  it("keeps zh-CN and en-US resource keys aligned", () => {
    expect(() => assertSdkworkCatalogLocaleParity(PAYMENT_ADMIN_I18N_CATALOG)).not.toThrow();
  });

  it("resolves Chinese copy for Payment admin workspace controls", () => {
    const messages = PAYMENT_ADMIN_I18N_CATALOG.resolveMessages("zh-CN");

    expect(messages.legacy.phrases["Payment operations monitor"]).toBe("支付运营监控");
    expect(messages.legacy.phrases["Replay webhook event"]).toBe("重放 Webhook 事件");
    expect(messages.legacy.tokens.failed).toBe("失败");
    expect(messages.legacy.phrases["Validate the saved credentials and provider adapter for "]).toBe(
      "校验已保存凭据与支付机构适配器：",
    );
    expect(messages.legacy.phrases["Delete sub-merchant "]).toBe("删除子商户 ");
    expect(messages.legacy.tokens.pay).toBe("支付");
    expect(messages.legacy.tokens["Showing "]).toBe("已显示 ");
  });
});
