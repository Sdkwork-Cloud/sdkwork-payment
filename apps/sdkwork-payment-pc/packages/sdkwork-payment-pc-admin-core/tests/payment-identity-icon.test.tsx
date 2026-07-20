import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { PaymentMethodIcon, PaymentProviderIcon, PaymentSceneIcon } from "../src";

afterEach(cleanup);

describe("payment identity icons", () => {
  it.each([
    ["stripe_card", "stripe"],
    ["alipay_qr", "alipay"],
    ["wechat_jsapi", "wechat_pay"],
    ["sandbox_test", "sandbox"],
  ])("renders a provider-toned icon for %s", (methodKey, providerCode) => {
    const { container } = render(
      <PaymentMethodIcon methodKey={methodKey} providerCode={providerCode} />,
    );

    expect(container.querySelector(`[data-method-key="${methodKey}"]`)).not.toBeNull();
    expect(container.querySelector("svg")).not.toBeNull();
  });

  it("supports provider and scene identities", () => {
    const { container } = render(
      <div>
        <PaymentProviderIcon providerCode="wechat_pay" />
        <PaymentSceneIcon sceneCode="mini_program" />
      </div>,
    );

    expect(container.querySelector('[data-provider="wechat_pay"]')).not.toBeNull();
    expect(container.querySelector('[data-scene="mini_program"]')).not.toBeNull();
  });
});
