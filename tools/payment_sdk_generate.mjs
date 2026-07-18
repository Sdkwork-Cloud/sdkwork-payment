#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const HTTP_METHODS = new Set(["get", "post", "put", "patch", "delete", "head", "options", "trace"]);
const SDK_FAMILY = "sdkwork-payment-backend-sdk";
const SDK_OWNER = "sdkwork-payment";
const API_AUTHORITY = "sdkwork-payment-backend-api";
const API_PREFIX = "/backend/v3/api";
const STANDARD_PROFILE = "sdkwork-v3";
const FIXED_SDK_VERSION = "0.1.0";

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const familyRoot = path.join(workspaceRoot, "sdks", SDK_FAMILY);
const sourceOpenapiPath = path.join(
  workspaceRoot,
  "apis",
  "backend-api",
  "payment",
  `${API_AUTHORITY}.openapi.yaml`,
);
const authorityOpenapiPath = path.join(familyRoot, "openapi", `${API_AUTHORITY}.openapi.yaml`);
const sdkgenOpenapiPath = path.join(familyRoot, "openapi", `${API_AUTHORITY}.sdkgen.yaml`);
const generatedRoot = path.join(
  familyRoot,
  `${SDK_FAMILY}-typescript`,
  "generated",
  "server-openapi",
);
const generatorBin = path.resolve(
  workspaceRoot,
  "..",
  "sdkwork-sdk-generator",
  "bin",
  "sdkgen.js",
);

function fail(message) {
  process.stderr.write(`[payment_sdk_generate] ${message}\n`);
  process.exit(1);
}

function readOpenapi(filePath) {
  if (!existsSync(filePath)) {
    throw new Error(`missing OpenAPI contract: ${filePath}`);
  }
  return JSON.parse(readFileSync(filePath, "utf8"));
}

function stableJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function cloneJson(value) {
  return JSON.parse(JSON.stringify(value));
}

function collectOperations(openapi) {
  const operations = [];
  for (const [pathKey, pathItem] of Object.entries(openapi.paths ?? {})) {
    for (const [method, operation] of Object.entries(pathItem ?? {})) {
      if (!HTTP_METHODS.has(method)) {
        continue;
      }
      operations.push({ method, operation, path: pathKey });
    }
  }
  return operations;
}

function validateOpenapi(openapi) {
  if (openapi.openapi !== "3.1.2") {
    throw new Error(`${API_AUTHORITY} must use OpenAPI 3.1.2`);
  }
  if (openapi.info?.["x-sdkwork-api-authority"] !== API_AUTHORITY) {
    throw new Error(`${API_AUTHORITY} authority metadata mismatch`);
  }
  const operations = collectOperations(openapi);
  if (operations.length === 0) {
    throw new Error(`${API_AUTHORITY} must declare operations`);
  }
  for (const { method, operation, path: operationPath } of operations) {
    if (!operationPath.startsWith(API_PREFIX)) {
      throw new Error(`${method.toUpperCase()} ${operationPath} must start with ${API_PREFIX}`);
    }
    if (operation["x-sdkwork-owner"] !== SDK_OWNER) {
      throw new Error(`${operation.operationId} must be owned by ${SDK_OWNER}`);
    }
    if (operation["x-sdkwork-api-authority"] !== API_AUTHORITY) {
      throw new Error(`${operation.operationId} authority metadata mismatch`);
    }
    if (!String(operation["x-sdkwork-permission"] ?? "").trim()) {
      throw new Error(`${operation.operationId} must declare x-sdkwork-permission`);
    }
  }
  return operations.length;
}

function materializeSdkgenOpenapi(openapi) {
  const derived = cloneJson(openapi);
  const responseComponents = derived.components?.responses ?? {};
  for (const { operation } of collectOperations(derived)) {
    for (const [statusCode, response] of Object.entries(operation.responses ?? {})) {
      if (!response || typeof response !== "object" || typeof response.$ref !== "string") {
        continue;
      }
      const prefix = "#/components/responses/";
      if (!response.$ref.startsWith(prefix)) {
        continue;
      }
      const componentName = response.$ref.slice(prefix.length);
      const component = responseComponents[componentName];
      if (!component) {
        throw new Error(`${operation.operationId} response ${statusCode} references missing ${response.$ref}`);
      }
      operation.responses[statusCode] = cloneJson(component);
    }
  }
  return derived;
}

function synchronizeOpenapi(openapi, sdkgenOpenapi, checkMode) {
  const sourceOpenapi = readFileSync(sourceOpenapiPath, "utf8");
  for (const [targetPath, expected] of [
    [authorityOpenapiPath, sourceOpenapi],
    [sdkgenOpenapiPath, stableJson(sdkgenOpenapi)],
  ]) {
    const current = existsSync(targetPath) ? readFileSync(targetPath, "utf8") : "";
    if (checkMode && current !== expected) {
      throw new Error(`${path.relative(workspaceRoot, targetPath)} is not synchronized with the source OpenAPI`);
    }
    if (!checkMode && current !== expected) {
      mkdirSync(path.dirname(targetPath), { recursive: true });
      writeFileSync(targetPath, expected, "utf8");
    }
  }
}

function validateManifest(checkMode, operationCount) {
  const manifestPath = path.join(familyRoot, "sdk-manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  if (manifest.sdkOwner !== SDK_OWNER || manifest.apiAuthority !== API_AUTHORITY) {
    throw new Error("sdk-manifest owner or authority mismatch");
  }
  if (!Array.isArray(manifest.sdkDependencies)) {
    throw new Error("sdk-manifest sdkDependencies must be explicit");
  }
  manifest.ownerOnlyOperationCount = operationCount;
  const language = manifest.languages?.find((item) => item.language === "typescript");
  if (!language) {
    throw new Error("sdk-manifest must declare the TypeScript workspace");
  }
  language.generationState = existsSync(path.join(generatedRoot, "src", "index.ts"))
    ? "generated"
    : "pending";
  if (!checkMode) {
    writeFileSync(manifestPath, stableJson(manifest), "utf8");
  } else if (language.generationState !== "generated") {
    throw new Error("TypeScript backend SDK has not been generated");
  }
}

function runSdkgen() {
  if (!existsSync(generatorBin)) {
    throw new Error(`standard SDK generator not found: ${generatorBin}`);
  }
  const result = spawnSync(
    "node",
    [
      generatorBin,
      "generate",
      "--input",
      sdkgenOpenapiPath,
      "--output",
      generatedRoot,
      "--name",
      SDK_FAMILY,
      "--type",
      "backend",
      "--language",
      "typescript",
      "--base-url",
      "http://127.0.0.1:8080",
      "--api-prefix",
      API_PREFIX,
      "--fixed-sdk-version",
      FIXED_SDK_VERSION,
      "--sdk-root",
      familyRoot,
      "--sdk-name",
      SDK_FAMILY,
      "--package-name",
      "sdkwork-payment-backend-sdk-generated-typescript",
      "--standard-profile",
      STANDARD_PROFILE,
    ],
    { cwd: familyRoot, stdio: "inherit" },
  );
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`sdkgen failed with exit code ${result.status}`);
  }
}

const checkMode = process.argv.includes("--check");

try {
  const openapi = readOpenapi(sourceOpenapiPath);
  const operationCount = validateOpenapi(openapi);
  const sdkgenOpenapi = materializeSdkgenOpenapi(openapi);
  synchronizeOpenapi(openapi, sdkgenOpenapi, checkMode);
  if (!checkMode) {
    runSdkgen();
  }
  validateManifest(checkMode, operationCount);
  process.stdout.write(
    `[payment_sdk_generate] ${checkMode ? "check passed" : "generation completed"} (${operationCount} operations)\n`,
  );
} catch (error) {
  fail(error instanceof Error ? error.message : String(error));
}
