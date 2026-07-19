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
    "006_upgrade_bootstrap_templates.sql",
    "005_production_sandbox_template.sql",
  ]);
  assert.deepEqual(manifest.profiles.production.common, manifest.profiles.standard.common);
  assert.deepEqual(manifest.profiles.development.common, [
    "001_payment_method_catalog.sql",
    "002_production_templates.sql",
    "006_upgrade_bootstrap_templates.sql",
    "003_development_templates.sql",
  ]);
  assert.deepEqual(manifest.profiles.test.common, [
    "001_payment_method_catalog.sql",
    "002_production_templates.sql",
    "006_upgrade_bootstrap_templates.sql",
    "004_test_templates.sql",
  ]);
});

test("payment seeds keep real PSP accounts gated and sandbox profiles operable", async () => {
  const [catalog, externalTemplates, productionSandbox, development, testProfile, upgrade] =
    await Promise.all([
      readSeed("common/001_payment_method_catalog.sql"),
      readSeed("common/002_production_templates.sql"),
      readSeed("common/005_production_sandbox_template.sql"),
      readSeed("common/003_development_templates.sql"),
      readSeed("common/004_test_templates.sql"),
      readSeed("common/006_upgrade_bootstrap_templates.sql"),
    ]);

  for (const methodKey of ["stripe_card", "alipay_qr", "wechat_native"]) {
    assert.match(catalog, new RegExp(`'${methodKey}'`));
  }
  assert.match(catalog, /'wechat_native'[^\n]*'active'/);
  assert.match(externalTemplates, /bootstrap-payment-channel-wechat-native[^\n]*'active'/);
  assert.match(productionSandbox, /'sandbox_test'[\s\S]*'inactive'/);
  assert.match(externalTemplates, /database:primary_secret/);
  assert.match(externalTemplates, /database:webhook_secret/);
  assert.match(externalTemplates, /database:certificate/);
  assert.match(externalTemplates, /mock-wechat-mch-id/);
  assert.match(externalTemplates, /mock-wechat-app-id/);
  assert.match(externalTemplates, /mock-wechat-merchant-serial-no/);
  assert.doesNotMatch(externalTemplates, /sk_live_|BEGIN (?:RSA )?PRIVATE KEY/);
  assert.match(development, /'sandbox_test'[\s\S]*'active'/);
  assert.match(testProfile, /'sandbox_test'[\s\S]*'active'/);
  assert.match(upgrade, /mock-wechat-mch-id/);
});

test("payment JSON seeds preserve PostgreSQL jsonb and SQLite portability", async () => {
  const seedFiles = [
    "common/001_payment_method_catalog.sql",
    "common/002_production_templates.sql",
    "common/003_development_templates.sql",
    "common/004_test_templates.sql",
    "common/005_production_sandbox_template.sql",
  ];

  for (const relativePath of seedFiles) {
    const sql = await readSeed(relativePath);
    assert.doesNotMatch(sql, /WITH\s+seed\s*\(/i, relativePath);
    assert.match(sql, /ON\s+CONFLICT\s+DO\s+NOTHING/i, relativePath);
  }

  const upgrade = await readSeed("common/006_upgrade_bootstrap_templates.sql");
  assert.doesNotMatch(upgrade, /json_(?:set|patch|extract)/i);
});
