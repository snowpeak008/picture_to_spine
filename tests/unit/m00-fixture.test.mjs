import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';

const sha=(bytes)=>createHash('sha256').update(bytes).digest('hex');
test('synthetic fixture is transparent PNG and exact ten-action registry',async()=>{
  const manifest=JSON.parse(await readFile('fixtures/m00/synthetic-character/manifest.json','utf8'));
  assert.deepEqual(manifest.actionKeys,['idle','run','jump','fall','dash','attack_01','attack_02','attack_03','hit','death']);
  for(const [name,expected] of Object.entries(manifest.outputHashes)){const bytes=await readFile(`fixtures/m00/synthetic-character/${name}`);assert.deepEqual([...bytes.subarray(0,8)],[137,80,78,71,13,10,26,10]);assert.equal(sha(bytes),expected);}
  assert.equal(manifest.realDomainClaimAllowed,false);assert.equal(manifest.approvalState,'UNAPPROVED_TEST_ONLY');
});

test('license policy contains fail-closed families',async()=>{
  const inventory=JSON.parse(await readFile('docs/compliance/F2S-SUPPLY-INVENTORY-001.json','utf8'));
  for(const family of ['AGPL','GPL','LGPL','MPL','SSPL','NC','Research-Only','unknown'])assert.ok(inventory.forbiddenFamilies.includes(family));
});
