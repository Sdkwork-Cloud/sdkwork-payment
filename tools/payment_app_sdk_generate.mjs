#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HTTP_METHODS = new Set(['get', 'post', 'put', 'patch', 'delete', 'head', 'options', 'trace']);
const SDK_FAMILY = 'sdkwork-payment-app-sdk';
const SDK_OWNER = 'sdkwork-payment';
const API_AUTHORITY = 'sdkwork-payment-app-api';
const API_PREFIX = '/app/v3/api';
const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const familyRoot = path.join(workspaceRoot, 'sdks', SDK_FAMILY);
const sourcePath = path.join(workspaceRoot, 'apis', 'app-api', 'payment', `${API_AUTHORITY}.openapi.yaml`);
const authorityPath = path.join(familyRoot, 'openapi', `${API_AUTHORITY}.openapi.json`);
const sdkgenPath = path.join(familyRoot, 'openapi', `${API_AUTHORITY}.sdkgen.json`);
const generatedRoot = path.join(familyRoot, `${SDK_FAMILY}-typescript`, 'generated', 'server-openapi');
const generatorBin = path.resolve(workspaceRoot, '..', 'sdkwork-sdk-generator', 'bin', 'sdkgen.js');
const checkMode = process.argv.includes('--check');

function stableJson(value) {
  return `${JSON.stringify(value, null, 2)}\n`;
}

function collectOperations(openapi) {
  return Object.entries(openapi.paths ?? {}).flatMap(([operationPath, pathItem]) =>
    Object.entries(pathItem ?? {})
      .filter(([method]) => HTTP_METHODS.has(method))
      .map(([method, operation]) => ({ method, operation, operationPath })),
  );
}

function validate(openapi) {
  if (openapi.openapi !== '3.1.2') throw new Error('OpenAPI must use 3.1.2');
  if (openapi.info?.['x-sdkwork-api-authority'] !== API_AUTHORITY) throw new Error('authority mismatch');
  const operations = collectOperations(openapi);
  for (const { method, operation, operationPath } of operations) {
    if (!operationPath.startsWith(API_PREFIX)) throw new Error(`${method} ${operationPath} has invalid prefix`);
    if (operation['x-sdkwork-owner'] !== SDK_OWNER) throw new Error(`${operation.operationId} owner mismatch`);
    if (operation['x-sdkwork-api-authority'] !== API_AUTHORITY) throw new Error(`${operation.operationId} authority mismatch`);
    if (!String(operation['x-sdkwork-permission'] ?? '').trim()) throw new Error(`${operation.operationId} missing permission`);
  }
  return operations.length;
}

function inlineResponseReferences(openapi) {
  const derived = structuredClone(openapi);
  const responses = derived.components?.responses ?? {};
  for (const { operation } of collectOperations(derived)) {
    for (const [status, response] of Object.entries(operation.responses ?? {})) {
      const prefix = '#/components/responses/';
      if (typeof response?.$ref === 'string' && response.$ref.startsWith(prefix)) {
        const resolved = responses[response.$ref.slice(prefix.length)];
        if (!resolved) throw new Error(`missing response component ${response.$ref}`);
        operation.responses[status] = structuredClone(resolved);
      }
    }
  }
  return derived;
}

function synchronize(target, content) {
  const current = existsSync(target) ? readFileSync(target, 'utf8') : '';
  if (checkMode && current !== content) throw new Error(`${path.relative(workspaceRoot, target)} is not synchronized`);
  if (!checkMode && current !== content) {
    mkdirSync(path.dirname(target), { recursive: true });
    writeFileSync(target, content, 'utf8');
  }
}

try {
  const source = readFileSync(sourcePath, 'utf8');
  const openapi = JSON.parse(source);
  const operationCount = validate(openapi);
  synchronize(authorityPath, stableJson(openapi));
  synchronize(sdkgenPath, stableJson(inlineResponseReferences(openapi)));
  if (!checkMode) {
    const result = spawnSync('node', [
      generatorBin,
      'generate',
      '--input', sdkgenPath,
      '--output', generatedRoot,
      '--name', SDK_FAMILY,
      '--type', 'app',
      '--language', 'typescript',
      '--base-url', 'http://127.0.0.1:8080',
      '--api-prefix', API_PREFIX,
      '--fixed-sdk-version', '0.1.0',
      '--sdk-root', familyRoot,
      '--sdk-name', SDK_FAMILY,
      '--package-name', 'sdkwork-payment-app-sdk-generated-typescript',
      '--standard-profile', 'sdkwork-v3',
    ], { cwd: familyRoot, stdio: 'inherit' });
    if (result.status !== 0) throw new Error(`sdkgen failed with exit code ${result.status}`);
  }
  const manifestPath = path.join(familyRoot, 'sdk-manifest.json');
  const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
  manifest.ownerOnlyOperationCount = operationCount;
  manifest.languages[0].generationState = existsSync(path.join(generatedRoot, 'src', 'index.ts')) ? 'generated' : 'pending';
  if (checkMode && manifest.languages[0].generationState !== 'generated') throw new Error('TypeScript SDK is not generated');
  if (!checkMode) writeFileSync(manifestPath, stableJson(manifest), 'utf8');
  process.stdout.write(`[payment_app_sdk_generate] ${checkMode ? 'check passed' : 'generation completed'} (${operationCount} operations)\n`);
} catch (error) {
  process.stderr.write(`[payment_app_sdk_generate] ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
}
