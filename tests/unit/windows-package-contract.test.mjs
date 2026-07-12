import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';

const validator = path.resolve('tools/windows/validate-json-contract.mjs');
const buildSchema = path.resolve('tools/windows/core-build-binding.schema.json');
const packageSchema = path.resolve('tools/windows/core-package-manifest.schema.json');
const sha = 'a'.repeat(64);

async function validate(schema, value) {
  const root = await mkdtemp(path.join(tmpdir(), 'f2s-json-contract-'));
  const file = path.join(root, 'value.json');
  try {
    await writeFile(file, JSON.stringify(value), 'utf8');
    return spawnSync(process.execPath, [validator, schema, file], { encoding: 'utf8' });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

const validBuildReceipt = () => ({
  schemaVersion: 'f2s-core-build-binding/1.0.0', target: 'x86_64-pc-windows-msvc',
  profile: 'release', cargoLocked: true, cargoOffline: true,
  executableSha256: sha, buildInputSha256: sha, toolchainFingerprintSha256: sha, sourceTreeSha256: sha,
  sourceFileCount: 10, cargoLockSha256: sha, nodeLockSha256: sha,
  uiBundleSha256: sha, uiBundleFileCount: 6
});

const validPackageManifest = () => ({
  schemaVersion: '1.0.0', product: 'FlashToSpine', version: '0.1.0',
  packageKind: 'windows-portable-core-internal', target: 'x86_64-pc-windows-msvc',
  entrypoint: 'FlashToSpine.exe',
  deterministicInputs: {
    cargoLocked: true, cargoOffline: true, nodeLock: 'package-lock.json', cargoLock: 'Cargo.lock',
    buildInputSha256: sha, toolchainFingerprintSha256: sha, sourceTreeSha256: sha, sourceFileCount: 10,
    cargoLockSha256: sha, nodeLockSha256: sha, uiBundleSha256: sha,
    uiBundleFileCount: 6, buildBindingSha256: sha
  },
  prerequisites: { webView2: 'SYSTEM_EVERGREEN_REQUIRED_UNVERIFIED' },
  capabilities: {
    coreBinary: 'BUILT_UNVERIFIED_CLEAN_VM', uiEmbedded: 'BUILT',
    spineEditor: 'EXTERNAL_NOT_INCLUDED', appContainerWorker: 'NOT_INCLUDED_UNVERIFIED',
    codeSignature: 'NOT_RUN_EXTERNAL'
  },
  security: { networkInstaller: false, elevationRequired: false, downloadsDependencies: false },
  files: [
    { path: 'FlashToSpine.exe', sha256: sha, bytes: 1 },
    { path: 'README.txt', sha256: sha, bytes: 1 },
    { path: 'build-binding.json', sha256: sha, bytes: 1 }
  ]
});

test('build receipt schema rejects false build provenance', async () => {
  assert.equal((await validate(buildSchema, validBuildReceipt())).status, 0);
  for (const mutate of [
    value => { value.profile = 'debug'; },
    value => { value.cargoLocked = false; },
    value => { value.cargoOffline = false; },
    value => { value.unexpected = 'field'; }
  ]) {
    const value = validBuildReceipt(); mutate(value);
    assert.notEqual((await validate(buildSchema, value)).status, 0);
  }
});

test('package schema rejects capability inflation and incomplete file lists', async () => {
  assert.equal((await validate(packageSchema, validPackageManifest())).status, 0);
  for (const mutate of [
    value => { value.capabilities.spineEditor = 'VERIFIED'; },
    value => { value.files.splice(1, 1); },
    value => { value.files[1] = value.files[0]; },
    value => { [value.files[0], value.files[1]] = [value.files[1], value.files[0]]; }
  ]) {
    const value = validPackageManifest(); mutate(value);
    assert.notEqual((await validate(packageSchema, value)).status, 0);
  }
});
