#!/usr/bin/env node
import { readdirSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const LOCALES_DIR = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'src', 'locales');
const BASE = 'en.json';
const PLACEHOLDER = /\{\{\s*([^}\s]+)\s*\}\}/g;

function flatten(node, prefix, out) {
  for (const [key, value] of Object.entries(node)) {
    const path = prefix ? `${prefix}.${key}` : key;
    if (value !== null && typeof value === 'object') flatten(value, path, out);
    else out.set(path, String(value));
  }
  return out;
}

function placeholders(text) {
  return [...text.matchAll(PLACEHOLDER)].map((m) => m[1]).sort().join(',');
}

function load(file) {
  return flatten(JSON.parse(readFileSync(join(LOCALES_DIR, file), 'utf8')), '', new Map());
}

const errors = [];
const base = load(BASE);

for (const file of readdirSync(LOCALES_DIR).filter((f) => f.endsWith('.json') && f !== BASE).sort()) {
  let keys;
  try {
    keys = load(file);
  } catch (e) {
    errors.push(`${file}: invalid JSON (${e.message})`);
    continue;
  }
  for (const [key, text] of base) {
    if (!keys.has(key)) errors.push(`${file}: missing key ${key}`);
    else if (placeholders(keys.get(key)) !== placeholders(text)) {
      errors.push(`${file}: placeholders differ from ${BASE} for ${key}`);
    }
  }
  for (const key of keys.keys()) {
    if (!base.has(key)) errors.push(`${file}: extra key ${key}`);
  }
}

if (errors.length > 0) {
  console.error(errors.join('\n'));
  console.error(`\n${errors.length} locale parity error(s)`);
  process.exit(1);
}
console.log(`Locale parity OK: ${readdirSync(LOCALES_DIR).length} files, ${base.size} keys each`);
