import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';

test('toolchain probe is local observation, never clean-vm verified',async()=>{
  const probe=JSON.parse(await readFile('evidence/M00/F2S-DEV-M00-001/F2S-WU-M00-001-01/probe.json','utf8'));
  assert.equal(probe.machine.runnerKind,'local-observation');assert.equal(probe.overallState,'OBSERVED_LOCAL');assert.ok(probe.tools.every(item=>item.state!=='VERIFIED'));
});

test('PSD probe roundtrips two named RGBA layers',async()=>{
  const report=JSON.parse(await readFile('evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-01/psd-roundtrip.json','utf8'));
  assert.equal(report.status,'PASS');assert.equal(report.parsed.channels,4);assert.deepEqual(report.parsed.layerNames,['body','weapon']);assert.equal(report.issues.length,0);
});

test('launcher source has no install, elevation, network or execution-policy mutation',()=>{
  const output=execFileSync('powershell.exe',['-NoProfile','-File','tools/launcher/dev-launch-selftest.ps1'],{encoding:'utf8'});
  assert.equal(JSON.parse(output).status,'PASS');
});
