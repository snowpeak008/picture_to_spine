import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('diagnostics export is host-wired, redacted, and does not overstate observations', async () => {
  const [host, writer, ui] = await Promise.all([
    readFile('apps/desktop/src-tauri/src/ipc_host.rs', 'utf8'),
    readFile('apps/desktop/src-tauri/src/diagnostics_export.rs', 'utf8'),
    readFile('apps/desktop-ui/src/features/diagnostics/DiagnosticsPage.tsx', 'utf8'),
  ]);
  assert.ok(host.includes('IpcMethod::DiagnosticsExport =>'));
  assert.equal(host.includes('IpcMethod::ProjectCommand | IpcMethod::JobCancel | IpcMethod::DiagnosticsExport'), false);
  for (const marker of ['RUST_FIXED_WHITELIST_DIAGNOSTICS', 'CAPABILITY_AVAILABLE · NOT_RUN_CURRENT_PROJECT', 'UNVERIFIED_EXCLUDED']) assert.ok(host.includes(marker), marker);
  for (const excluded of ['image-bytes', 'prompt-text', 'credentials', 'absolute-paths', 'private-endpoint-origin']) assert.ok(writer.includes(excluded), excluded);
  assert.ok(ui.includes('data?.evidence.networkCallCount'));
  assert.equal(ui.includes("data?.networkCallCount === 0 ? 'OBSERVED_RUNTIME'"), false);
});
