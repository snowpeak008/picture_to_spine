import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';

const read = path => readFile(path, 'utf8');

test('Windows Core build is locked, offline, and produces a release binary', async () => {
  const packageJson = JSON.parse(await read('package.json'));
  const source = await read('tools/windows/build-core.ps1');
  assert.match(packageJson.scripts['build:core'], /tools\/windows\/build-core\.ps1/);
  assert.match(source, /cargo build -p flash-to-spine-desktop --release --locked --offline --target \$target/);
  assert.match(source, /x86_64-pc-windows-msvc/);
  assert.doesNotMatch(source, /npm (?:ci|install)|cargo install|Invoke-WebRequest|curl /i);
});

test('portable package and launcher do not install, elevate, download, or claim external capabilities', async () => {
  const files = [
    'tools/windows/package-core.ps1',
    'tools/windows/verify-core-package.ps1',
    'tools/launcher/dev-launch.ps1',
    'FlashToSpine-开发入口.cmd'
  ];
  const source = (await Promise.all(files.map(read))).join('\n');
  assert.doesNotMatch(source, /Invoke-WebRequest|curl |npm (?:ci|install)|cargo install|-Verb RunAs|Set-ExecutionPolicy|signtool sign/i);
  assert.match(source, /NOT_RUN_EXTERNAL/);
  assert.match(source, /SYSTEM_(?:EVERGREEN_REQUIRED_UNVERIFIED|PREREQUISITE_UNVERIFIED)/);
  assert.match(source, /NOT_INCLUDED_UNVERIFIED/);
});

test('release verification packages and smokes the Core candidate', async () => {
  const packageJson = JSON.parse(await read('package.json'));
  assert.match(packageJson.scripts['release:verify'], /npm run package:core/);
  assert.match(packageJson.scripts['release:verify'], /npm run test:package/);
  const main = await read('apps/desktop/src-tauri/src/main.rs');
  assert.match(main, /--smoke/);
  assert.match(main, /NOT_PROBED_SYSTEM_PREREQUISITE/);
  assert.match(main, /F2S_BUILD_INPUT_SHA256/);
  assert.match(main, /buildInputSha256/);
  assert.match(main, /std::process::exit\(1\)/);
});

test('portable package binds the binary to current locked source inputs', async () => {
  const packageSource = await read('tools/windows/package-core.ps1');
  const verifySource = await read('tools/windows/verify-core-package.ps1');
  const buildSource = await read('tools/windows/build-core.ps1');
  const bindingSource = await read('tools/windows/build-input-binding.ps1');
  const validationSource = await read('tools/windows/core-package-validation.ps1');
  const schema = JSON.parse(await read('tools/windows/core-package-manifest.schema.json'));
  for (const field of [
    'buildInputSha256',
    'toolchainFingerprintSha256',
    'sourceTreeSha256',
    'sourceFileCount',
    'cargoLockSha256',
    'nodeLockSha256',
    'uiBundleSha256',
    'uiBundleFileCount',
    'buildBindingSha256'
  ]) {
    assert.match(`${packageSource}\n${validationSource}`, new RegExp(field));
    assert.match(`${verifySource}\n${bindingSource}\n${validationSource}`, new RegExp(field));
    assert.ok(schema.properties.deterministicInputs.required.includes(field));
  }
  assert.match(buildSource, /Get-F2sSourceInputBinding/);
  assert.match(buildSource, /Get-F2sUiBundleBinding/);
  assert.match(buildSource, /Isolated source inputs changed while the UI was building/);
  assert.match(buildSource, /Isolated source inputs changed while cargo was compiling/);
  assert.match(buildSource, /FlashToSpine-build-snapshots/);
  assert.match(buildSource, /npm\.cmd ci --offline --ignore-scripts/);
  assert.doesNotMatch(buildSource, /ItemType Junction/);
  assert.match(buildSource, /CARGO_TARGET_DIR=Join-Path \$snapshot 'cargo-target'/);
  assert.match(buildSource, /f2s-build-transaction\/1\.0\.0/);
  assert.match(buildSource, /f2s-core-build-binding\/1\.0\.0/);
  assert.match(packageSource, /Staged Release EXE does not match its build-time receipt/);
  assert.match(packageSource, /stale binaries cannot be packaged/);
  assert.match(validationSource, /Packaged binary is not bound to the current source inputs/);
  assert.match(validationSource, /Packaged EXE does not match its build-time receipt/);
  assert.match(validationSource, /Packaged smoke build-input binding mismatch/);
  assert.match(validationSource, /validate-json-contract\.mjs/);
});

test('source binding detects hidden Cargo inputs and rejects profile overrides', () => {
  const result = spawnSync('powershell.exe', [
    '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass',
    '-File', 'tools/windows/build-input-binding-selftest.ps1'
  ], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const report = JSON.parse(result.stdout.trim().split(/\r?\n/).at(-1));
  assert.deepEqual(report, { status: 'PASS', hiddenInputDetected: true, profileOverrideRejected: true, nodeOverrideRejected: true, buildBinaryOverrideRejected: true, prefixOverrideRejected: true });
});

test('physical package validator rejects hidden nested capability payloads', () => {
  const result = spawnSync('powershell.exe', [
    '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass',
    '-File', 'tools/windows/package-tree-selftest.ps1'
  ], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const report = JSON.parse(result.stdout.trim().split(/\r?\n/).at(-1));
  assert.deepEqual(report, { status: 'PASS', hiddenNestedPayloadRejected: true });
});

test('package and verify reject an interrupted build transaction marker', () => {
  const result = spawnSync('powershell.exe', [
    '-NoLogo', '-NoProfile', '-ExecutionPolicy', 'Bypass',
    '-File', 'tools/windows/build-marker-selftest.ps1'
  ], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
  const report = JSON.parse(result.stdout.trim().split(/\r?\n/).at(-1));
  assert.deepEqual(report, { status: 'PASS', interruptedBuildRejected: true });
});
