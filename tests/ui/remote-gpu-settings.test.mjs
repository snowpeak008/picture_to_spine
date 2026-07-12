import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('private remote GPU settings are local-only and expose truthful external state', async () => {
  const [ui, host, profileSchema] = await Promise.all([
    readFile('apps/desktop-ui/src/features/settings/RemoteGpuSettings.tsx', 'utf8'),
    readFile('apps/desktop/src-tauri/src/ipc_host.rs', 'utf8'),
    readFile('schemas/src/remote-gpu.schema.json', 'utf8').then(JSON.parse),
  ]);
  const methodVariants = [
    ['remoteGpu.status', 'RemoteGpuStatus'],
    ['remoteGpu.importProfile', 'RemoteGpuImportProfile'],
    ['remoteGpu.disable', 'RemoteGpuDisable'],
  ];
  for (const [method, variant] of methodVariants) {
    assert.ok(ui.includes(`'${method}'`), method);
    assert.ok(host.includes(`IpcMethod::${variant}`), method);
  }
  assert.ok(ui.includes('NOT_RUN / EXTERNAL'));
  assert.ok(ui.includes('网络尝试数'));
  assert.ok(ui.includes('不写入项目'));
  assert.equal(/fetch\(|XMLHttpRequest|WebSocket/.test(ui), false);
  assert.equal(profileSchema.additionalProperties, false);
  assert.equal(host.includes('"networkAttemptCount": 0'), true);
  assert.equal(host.includes('parse_remote_gpu_profile(&bytes)?'), true);
  assert.equal(host.includes('profile.validate_configuration()?'), true);
});

test('Rust, TypeScript and JSON Schema IPC method contracts stay aligned', async () => {
  const [rust, typescript, schema] = await Promise.all([
    readFile('crates/application/src/ports/ipc.rs', 'utf8'),
    readFile('apps/desktop-ui/src/native/ipc.ts', 'utf8'),
    readFile('schemas/src/ipc.schema.json', 'utf8').then(JSON.parse),
  ]);
  const rustMethods = [...rust.matchAll(/#\[serde\(rename = "([^"]+)"\)\]/g)].map(match => match[1]).sort();
  const unionLine = typescript.split('\n')[0];
  const typescriptMethods = [...unionLine.matchAll(/'([^']+)'/g)].map(match => match[1]).sort();
  const schemaMethods = [...schema.$defs.Request.properties.method.enum].sort();
  assert.deepEqual(typescriptMethods, rustMethods);
  assert.deepEqual(schemaMethods, rustMethods);
});
