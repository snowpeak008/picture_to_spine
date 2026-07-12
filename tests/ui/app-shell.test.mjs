import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('UI source exposes human gates and exact action keys',async()=>{const source=await readFile('apps/desktop-ui/src/app/AppShell.tsx','utf8');for(const marker of ['批准母版','分层与素材','Rig 工作台','关键姿势','attack_01','attack_02','attack_03','Spine 4.2.43'])assert.ok(source.includes(marker),marker);assert.equal(source.includes('attack_1\''),false);});
test('UI source does not import Tauri or network clients',async()=>{const files=['apps/desktop-ui/src/app/AppShell.tsx','apps/desktop-ui/src/features/diagnostics/DiagnosticsPage.tsx','apps/desktop-ui/src/main.tsx'];for(const file of files){const source=await readFile(file,'utf8');assert.equal(/@tauri-apps|fetch\(|XMLHttpRequest|WebSocket/.test(source),false,file);}});
