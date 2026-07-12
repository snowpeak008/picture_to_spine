import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('ProjectHead schema exposes both explicit legacy and sealed production shapes', async () => {
  const schema = JSON.parse(await readFile('schemas/src/storage.schema.json', 'utf8'));
  assert.equal(schema.additionalProperties, false);
  for (const field of ['keyId', 'previousHeadMac', 'headMac']) {
    assert.ok(schema.properties[field], field);
  }
  assert.equal(schema.oneOf.length, 2);
  const serialized = JSON.stringify(schema);
  assert.ok(serialized.includes('Legacy unsigned head'));
  assert.ok(serialized.includes('Production HMAC-sealed head'));
  assert.ok(serialized.includes('previousHeadMac'));
  assert.ok(serialized.includes('headRevision'));
});
