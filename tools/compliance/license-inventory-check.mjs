#!/usr/bin/env node
import { readFile, access, mkdir, writeFile } from 'node:fs/promises';
import { constants } from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

const root = process.cwd();
const inventoryPath = path.join(root, 'docs/compliance/F2S-SUPPLY-INVENTORY-001.json');
const outputArg = process.argv.find((arg) => arg.startsWith('--output='));
const outputPath = path.resolve(root, outputArg?.slice('--output='.length) ?? 'evidence/M00/F2S-DEV-M00-002/F2S-WU-M00-002-02/license-check.json');
const forbidden = /(^|[-.])(AGPL|GPL|LGPL|MPL|SSPL|BUSL)([-.]|$)|NonCommercial|Research|\bNC\b|unknown/i;

const inventory = JSON.parse(await readFile(inventoryPath, 'utf8'));
const allowed = new Set(inventory.allowedProductionSpdx);
const findings = [];

for (const item of inventory.items.toSorted((a, b) => a.id.localeCompare(b.id))) {
  for (const field of ['id', 'category', 'name', 'version', 'source', 'licenseExpression', 'licenseEvidence', 'packageInclusion', 'reviewState']) {
    if (item[field] === undefined || item[field] === '') findings.push({ code: 'MISSING_FIELD', itemId: item.id ?? null, field });
  }
  if (item.reviewState !== 'allowed') findings.push({ code: 'NOT_ALLOWED', itemId: item.id });
  if (forbidden.test(item.licenseExpression)) findings.push({ code: 'FORBIDDEN_LICENSE', itemId: item.id, licenseExpression: item.licenseExpression });
  if (item.packageInclusion === 'core' && !allowed.has(item.licenseExpression) && item.licenseExpression !== 'LicenseRef-Proprietary-Internal') {
    findings.push({ code: 'CORE_LICENSE_NOT_ALLOWLISTED', itemId: item.id, licenseExpression: item.licenseExpression });
  }
  if (item.licenseEvidence && !item.licenseEvidence.includes('ownership record')) {
    const evidencePath = path.resolve(root, item.licenseEvidence);
    try { await access(evidencePath, constants.R_OK); } catch { findings.push({ code: 'LICENSE_EVIDENCE_MISSING', itemId: item.id, path: item.licenseEvidence }); }
  }
}

let npmClosureCount = 0;
let cargoClosureCount = 0;
const packageJsonPath = path.join(root, 'package.json');
try {
  const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
  const lockedNames = Object.keys({ ...(packageJson.dependencies ?? {}), ...(packageJson.optionalDependencies ?? {}) });
  for (const name of lockedNames) {
    if (!inventory.items.some((item) => item.name === name || item.id === `npm:${name}`)) findings.push({ code: 'RUNTIME_DEPENDENCY_UNREGISTERED', name });
  }
} catch (error) {
  if (error.code !== 'ENOENT') findings.push({ code: 'PACKAGE_JSON_INVALID', message: error.message });
}

try {
  const lock = JSON.parse(await readFile(path.join(root, 'package-lock.json'), 'utf8'));
  for (const [lockPath, item] of Object.entries(lock.packages ?? {})) {
    if (!lockPath.startsWith('node_modules/') || item.link) continue;
    npmClosureCount += 1;
    const license = item.license;
    if (!license) findings.push({ code: 'NPM_LICENSE_MISSING', lockPath, version: item.version ?? null });
    else if (forbidden.test(license)) findings.push({ code: 'NPM_FORBIDDEN_LICENSE', lockPath, version: item.version, license });
    if (!item.integrity || !item.resolved) findings.push({ code: 'NPM_PROVENANCE_MISSING', lockPath, version: item.version ?? null });
  }
} catch (error) {
  findings.push({ code: 'PACKAGE_LOCK_MISSING_OR_INVALID', message: error.message });
}

try {
  const metadata = JSON.parse(execFileSync('cargo', ['metadata', '--locked', '--format-version', '1'], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 }));
  for (const item of metadata.packages) {
    if (!item.source?.startsWith('registry+')) continue;
    cargoClosureCount += 1;
    if (!item.license) findings.push({ code: 'CARGO_LICENSE_MISSING', name: item.name, version: item.version });
    else if (forbidden.test(item.license)) findings.push({ code: 'CARGO_FORBIDDEN_LICENSE', name: item.name, version: item.version, license: item.license });
  }
} catch (error) {
  findings.push({ code: 'CARGO_METADATA_FAILED', message: error.message });
}

const report = {
  schemaVersion: '1.0.0',
  policyId: inventory.policyId,
  status: findings.length === 0 ? 'PASS' : 'FAIL',
  inventorySha256: createHash('sha256').update(JSON.stringify(inventory)).digest('hex'),
  checkedItems: inventory.items.length,
  npmClosureCount,
  cargoClosureCount,
  findings: findings.toSorted((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b)))
};
await mkdir(path.dirname(outputPath), { recursive: true });
await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);
if (findings.length) process.exitCode = 2;
