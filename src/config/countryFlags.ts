// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Loaded on demand: some 250 Twemoji flags would otherwise all land in the main chunk.

const FLAG_LOADERS = import.meta.glob<string>('../../node_modules/@twemoji/svg/1f1{e,f}?-1f1{e,f}?.svg', {
  query: '?url',
  import: 'default',
});

const REGIONAL_INDICATOR_A = 0x1f1e6;
const LETTER_A = 'A'.charCodeAt(0);

function flagFile(code: string): string {
  const points = [...code.toUpperCase()].map((c) => (REGIONAL_INDICATOR_A + c.charCodeAt(0) - LETTER_A).toString(16));
  return `../../node_modules/@twemoji/svg/${points.join('-')}.svg`;
}

/** Bundled flag of an ISO 3166-1 alpha-2 code; undefined when Twemoji has none (e.g. AN). */
export async function loadCountryFlag(code: string): Promise<string | undefined> {
  const load = code.length === 2 ? FLAG_LOADERS[flagFile(code)] : undefined;
  return load ? load() : undefined;
}
