import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import path from 'node:path';

const probePath = path.resolve('tools/windows/webview2-startup-probe.ps1');
const schemaPath = path.resolve('tools/windows/webview2-startup-probe.schema.json');
const validatorPath = path.resolve('tools/windows/validate-json-contract.mjs');
const desktopMainPath = path.resolve('apps/desktop/src-tauri/src/main.rs');

const validReport = () => ({
  schemaVersion: 'f2s-webview2-local-startup-probe/1.0.0',
  status: 'PASS',
  runtimeScope: 'LOCAL_RUNTIME_ONLY',
  cleanVm: false,
  failureCode: 'NONE',
  executable: {
    name: 'FlashToSpineLauncher.exe',
    sha256: 'a'.repeat(64),
    reparsePoint: false
  },
  startup: { hidden: true, waitSeconds: 5 },
  windowContract: {
    expectedTopLevelClass: 'FlashToSpineWebView',
    expectedTopLevelTitle: 'FlashToSpine Production Assist',
    topLevelMatched: true,
    chromeWidgetChildFound: true,
    renderWidgetChildFound: true,
    processResponding: true
  },
  shutdown: {
    wmCloseSent: true,
    gracefulExit: true,
    forcedCleanupRequired: false,
    cleanupSucceeded: true
  }
});

async function validate(value) {
  const root = await mkdtemp(path.join(tmpdir(), 'f2s-webview-probe-contract-'));
  const valuePath = path.join(root, 'report.json');
  try {
    await writeFile(valuePath, JSON.stringify(value), 'utf8');
    return spawnSync(process.execPath, [validatorPath, schemaPath, valuePath], { encoding: 'utf8' });
  } finally {
    await rm(root, { recursive: true, force: true });
  }
}

test('WebView2 local probe PowerShell parses without execution', () => {
  const command = [
    '$tokens = $null',
    '$errors = $null',
    `[void][System.Management.Automation.Language.Parser]::ParseFile('${probePath.replaceAll("'", "''")}', [ref]$tokens, [ref]$errors)`,
    'if ($errors.Count -gt 0) { $errors | ForEach-Object { [Console]::Error.WriteLine($_.Message) }; exit 1 }'
  ].join('; ');
  const result = spawnSync('powershell.exe', ['-NoLogo', '-NoProfile', '-Command', command], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr || result.stdout);
});

test('WebView2 probe statically binds startup, window, responsiveness, and cleanup contracts', async () => {
  const source = await readFile(probePath, 'utf8');
  assert.match(source, /FlashToSpineLauncher\.exe/);
  assert.match(source, /ProcessWindowStyle\]::Hidden/);
  assert.match(source, /Start-Sleep -Seconds \$waitSeconds/);
  assert.match(source, /FileAttributes\]::ReparsePoint/);
  assert.match(source, /Get-FileHash[^\n]+SHA256/);
  assert.match(source, /FlashToSpineWebView/);
  assert.match(source, /FlashToSpine Production Assist/);
  assert.match(source, /Chrome_WidgetWin_\*/);
  assert.match(source, /Chrome_RenderWidgetHostHWND/);
  assert.match(source, /\.Responding/);
  assert.match(source, /WM_CLOSE/);
  assert.match(source, /finally\s*\{/);
  assert.match(source, /taskkill\.exe/);
  assert.match(source, /LOCAL_RUNTIME_ONLY/);
  assert.match(source, /cleanVm = \$false/);
});

test('probe report schema accepts a local pass and rejects sensitive or inflated fields', async () => {
  assert.equal((await validate(validReport())).status, 0);

  for (const mutate of [
    value => { value.cleanVm = true; },
    value => { value.runtimeScope = 'CLEAN_VM'; },
    value => { value.executable.path = 'C:\\Users\\someone\\FlashToSpineLauncher.exe'; },
    value => { value.pid = 1234; },
    value => { value.username = 'someone'; },
    value => { value.startup.waitSeconds = 1; }
  ]) {
    const value = validReport();
    mutate(value);
    assert.notEqual((await validate(value)).status, 0);
  }
});

test('local GUI probe is opt-in and excluded from headless release verification', async () => {
  const packageJson = JSON.parse(await readFile('package.json', 'utf8'));
  assert.match(packageJson.scripts['test:webview-local'], /webview2-startup-probe\.ps1 -Output JSON/);
  assert.doesNotMatch(packageJson.scripts.test, /test:webview-local|webview2-startup-probe/);
  assert.doesNotMatch(packageJson.scripts['release:verify'], /test:webview-local|webview2-startup-probe/);
});

test('desktop host keeps WebView2 user data in the private local application root', async () => {
  const source = await readFile(desktopMainPath, 'utf8');
  assert.match(source, /CreateCoreWebView2EnvironmentWithOptions/);
  assert.match(source, /var_os\("LOCALAPPDATA"\)/);
  assert.match(source, /\.join\("FlashToSpine"\)\s*\.join\("WebView2"\)/);
  assert.doesNotMatch(source, /CreateCoreWebView2Environment\(&handler\)/);
});

test('desktop host admits only its exact NavigateToString document and publishes readiness after DOM verification', async () => {
  const source = await readFile(desktopMainPath, 'utf8');
  assert.match(source, /WINDOW_TITLE_STARTING/);
  assert.match(source, /NAVIGATE_TO_STRING_URI_PREFIX/);
  assert.match(source, /BASE64_STANDARD\.encode\(html\.as_bytes\(\)\)/);
  assert.match(source, /uri == "about:blank" \|\| uri == trusted_document_uri/);
  assert.doesNotMatch(source, /starts_with\([^\n]*data:/);
  assert.match(source, /document\.querySelector\('\.app-shell'\)/);

  const navigationIndex = source.indexOf('navigate_to_string_and_verify(&html)?');
  const readyTitleIndex = source.indexOf('set_title(WINDOW_TITLE_READY)?');
  assert.notEqual(navigationIndex, -1);
  assert.notEqual(readyTitleIndex, -1);
  assert.ok(navigationIndex < readyTitleIndex, 'ready title must be published only after navigation and DOM verification');
});
