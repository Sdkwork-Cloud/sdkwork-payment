import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const seedRoot = path.join(root, "database", "seeds");

async function readSeed(relativePath) {
  return readFile(path.join(seedRoot, relativePath), "utf8");
}

test("payment seed profiles select a complete and safe environment template", async () => {
  const manifest = JSON.parse(await readSeed("seed.manifest.json"));

  assert.deepEqual(manifest.profiles.standard.common, [
    "001_payment_method_catalog.sql",
    "002_production_templates.sql",
    "005_production_sandbox_template.sql",
  ]);
  assert.deepEqual(manifest.profiles.production.common, manifest.profiles.standard.common);
  assert.deepEqual(manifest.profiles.development.common, [
    "001_payment_method_catalog.sql",
    "002_production_templates.sql",
    "003_development_templates.sql",
  ]);
  assert.deepEqual(manifest.profiles.test.common, [
    "001_payment_method_catalog.sql",
    "002_production_templates.sql",
    "004_test_templates.sql",
  ]);
});

test("payment seeds keep real PSP templates inactive and sandbox profiles operable", async () => {
  const [catalog, externalTemplates, productionSandbox, development, testProfile] = await Promise.all([
    readSeed("common/001_payment_method_catalog.sql"),
    readSeed("common/002_production_templates.sql"),
    readSeed("common/005_production_sandbox_template.sql"),
    readSeed("common/003_development_templates.sql"),
    readSeed("common/004_test_templates.sql"),
  ]);

  for (const methodKey of ["stripe_card", "alipay_qr", "wechat_native"]) {
    assert.match(catalog, new RegExp(`'${methodKey}'`));
  }
  assert.match(productionSandbox, /'sandbox_test'[\s\S]*'inactive'/);
  assert.match(externalTemplates, /SDKWORK_PAYMENT_STRIPE_SECRET_KEY/);
  assert.match(externalTemplates, /SDKWORK_PAYMENT_ALIPAY_PRIVATE_KEY/);
  assert.match(externalTemplates, /SDKWORK_PAYMENT_WECHAT_PAY_API_V3_KEY/);
  assert.doesNotMatch(externalTemplates, /sk_live_|BEGIN (?:RSA )?PRIVATE KEY/);
  assert.match(development, /'sandbox_test'[\s\S]*'active'/);
  assert.match(testProfile, /'sandbox_test'[\s\S]*'active'/);
});
