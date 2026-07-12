import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const read = path => readFile(new URL(`../../${path}`, import.meta.url), 'utf8');

test('Spine CLI UI and IPC expose an asynchronous redacted external workflow', async () => {
  const [ipc, exportUi, settingsUi, host, schema] = await Promise.all([
    read('apps/desktop-ui/src/native/ipc.ts'),
    read('apps/desktop-ui/src/features/export/ExportWorkspace.tsx'),
    read('apps/desktop-ui/src/features/settings/SpineCliSettings.tsx'),
    read('apps/desktop/src-tauri/src/spine_cli_host.rs'),
    read('schemas/src/ipc.schema.json'),
  ]);
  for (const method of ['spineCli.status', 'spineCli.selectAndAssess', 'spineCli.clear', 'spineCli.job.start', 'spineCli.job.status']) {
    assert.ok(ipc.includes(method), method);
    assert.ok(schema.includes(method), method);
  }
  for (const operation of ['IMPORT_PROJECT', 'PACK_ATLAS', 'EXPORT_BINARY']) {
    assert.ok(exportUi.includes(operation), operation);
    assert.ok(host.includes(operation), operation);
  }
  assert.ok(exportUi.includes('cliJobTerminal'));
  assert.ok(exportUi.includes('outputPathToken'));
  assert.ok(settingsUi.includes('不捆绑、不下载、不读取激活信息'));
  assert.ok(host.includes('absolutePathReturnedToWebView'));
  assert.ok(host.includes('prepare_consent_binding'));
  assert.ok(host.includes('authorizes_proprietary_output'));
  assert.ok(!exportUi.includes('canonicalExecutable'));
});
