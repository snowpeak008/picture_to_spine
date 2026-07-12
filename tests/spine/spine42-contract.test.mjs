import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { createHash } from 'node:crypto';
const sha256=(bytes)=>createHash('sha256').update(bytes).digest('hex');

test('Spine target and capability manifest are exact 4.2.43',async()=>{
  const skeletonBytes=await readFile('fixtures/m00/spine42-probe/skeleton.json');
  const rigBytes=await readFile('fixtures/m00/spine42-probe/rig-ir.json');
  const skeleton=JSON.parse(skeletonBytes);
  const rig=JSON.parse(rigBytes);
  const cap=JSON.parse(await readFile('fixtures/m00/spine42-probe/capability-manifest.json','utf8'));
  assert.equal(skeleton.skeleton.spine,'4.2.43');assert.equal(cap.capabilityId,'F2S-SPINE-CAP-4.2.43-001');assert.deepEqual(rig.timeBase,{numerator:1,denominator:30000});assert.equal(cap.staticContractStatus,'VERIFIED');assert.equal(cap.editorRoundTripStatus,'EXTERNAL');
  for(const writer of ['.atlas','.spine','.skel'])assert.ok(cap.forbiddenBuiltinWriters.includes(writer));
  assert.equal(cap.sourceHashes['rig-ir.json'],sha256(rigBytes));
  assert.equal(cap.sourceHashes['skeleton.json'],sha256(skeletonBytes));
  for(const [relative,expected] of Object.entries(cap.fixtureHashes)){
    assert.equal(sha256(await readFile(path.join('fixtures/m00/spine42-probe',relative))),expected,`fixture hash drift: ${relative}`);
  }
});

test('all attachment paths stay below the fixture root',async()=>{
  const skeleton=JSON.parse(await readFile('fixtures/m00/spine42-probe/skeleton.json','utf8'));
  const paths=Object.values(skeleton.skins[0].attachments).flatMap(slot=>Object.values(slot).map(item=>item.path));
  for(const item of paths){assert.equal(path.isAbsolute(item),false);assert.equal(item.includes('..'),false);}
});

test('external evidence remains NOT_RUN without user lease',async()=>{
  const report=JSON.parse(await readFile('evidence/M00/F2S-DEV-M00-005/F2S-WU-M00-005-03/editor-roundtrip.json','utf8'));
  assert.equal(report.status,'NOT_RUN');assert.equal(report.capabilityState,'EXTERNAL');assert.equal(report.exactVersion,'4.2.43');
});
