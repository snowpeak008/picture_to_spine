#!/usr/bin/env node
import { readFile } from 'node:fs/promises';

const [schemaPath, valuePath] = process.argv.slice(2);
if (!schemaPath || !valuePath) {
  process.stderr.write('usage: validate-json-contract.mjs <schema.json> <value.json>\n');
  process.exit(2);
}

const schemaRoot = JSON.parse(await readFile(schemaPath, 'utf8'));
const value = JSON.parse(await readFile(valuePath, 'utf8'));
const errors = [];
const typeOf = item => Array.isArray(item) ? 'array' : item === null ? 'null' : typeof item;
const equal = (left, right) => JSON.stringify(left) === JSON.stringify(right);
const resolveRef = ref => {
  if (!ref.startsWith('#/')) throw new Error(`only local JSON pointers are supported: ${ref}`);
  return ref.slice(2).split('/').reduce((node, part) => node[part.replaceAll('~1', '/').replaceAll('~0', '~')], schemaRoot);
};

function validate(schema, item, path = '$') {
  if (schema === false) { errors.push(`${path}: value is forbidden`); return; }
  if (schema === true) return;
  if (schema.$ref) validate(resolveRef(schema.$ref), item, path);
  for (const branch of schema.allOf ?? []) validate(branch, item, path);
  if ('const' in schema && !equal(item, schema.const)) errors.push(`${path}: expected const ${JSON.stringify(schema.const)}`);
  if (schema.type) {
    const actual = typeOf(item);
    const valid = schema.type === 'integer' ? Number.isInteger(item) : actual === schema.type;
    if (!valid) { errors.push(`${path}: expected ${schema.type}, got ${actual}`); return; }
  }
  if (typeof item === 'string') {
    if (schema.minLength !== undefined && item.length < schema.minLength) errors.push(`${path}: shorter than minLength`);
    if (schema.pattern && !(new RegExp(schema.pattern).test(item))) errors.push(`${path}: pattern mismatch`);
  }
  if (typeof item === 'number' && schema.minimum !== undefined && item < schema.minimum) errors.push(`${path}: below minimum`);
  if (Array.isArray(item)) {
    if (schema.minItems !== undefined && item.length < schema.minItems) errors.push(`${path}: too few items`);
    if (schema.maxItems !== undefined && item.length > schema.maxItems) errors.push(`${path}: too many items`);
    if (schema.uniqueItems && new Set(item.map(entry => JSON.stringify(entry))).size !== item.length) errors.push(`${path}: duplicate items`);
    const prefix = schema.prefixItems ?? [];
    prefix.forEach((entrySchema, index) => { if (index < item.length) validate(entrySchema, item[index], `${path}[${index}]`); });
    if (schema.items === false && item.length > prefix.length) errors.push(`${path}: additional array items are forbidden`);
    if (schema.items && schema.items !== true && prefix.length === 0) item.forEach((entry, index) => validate(schema.items, entry, `${path}[${index}]`));
  }
  if (item && !Array.isArray(item) && typeof item === 'object') {
    for (const key of schema.required ?? []) if (!(key in item)) errors.push(`${path}.${key}: required property missing`);
    for (const [key, child] of Object.entries(schema.properties ?? {})) if (key in item) validate(child, item[key], `${path}.${key}`);
    if (schema.additionalProperties === false) {
      const allowed = new Set(Object.keys(schema.properties ?? {}));
      for (const key of Object.keys(item)) if (!allowed.has(key)) errors.push(`${path}.${key}: additional property forbidden`);
    }
  }
}

validate(schemaRoot, value);
if (errors.length) {
  process.stderr.write(`${errors.join('\n')}\n`);
  process.exit(1);
}
process.stdout.write(JSON.stringify({ status: 'PASS', schema: schemaRoot.$id ?? schemaPath }) + '\n');
