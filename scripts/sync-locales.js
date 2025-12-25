#!/usr/bin/env node

/**
 * Script to sync translation files with the English source of truth.
 * Finds missing keys in each language file and translates them using AI.
 */

import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const localesDir = path.resolve(__dirname, '../src/locales');
const sourceFile = path.join(localesDir, 'en.json');

// LibreTranslate configuration
const LIBRE_TRANSLATE_API = process.env.LIBRE_TRANSLATE_API || 'https://libretranslate.com';
const LIBRE_TRANSLATE_API_KEY = process.env.LIBRE_TRANSLATE_API_KEY || '';

// Locale code mapping to LibreTranslate language codes
const LOCALE_CODE_MAP = {
  'de': 'de',    // German
  'es': 'es',    // Spanish
  'fr': 'fr',    // French
  'it': 'it',    // Italian
  'ja': 'ja',    // Japanese
  'ko': 'ko',    // Korean
  'nl': 'nl',    // Dutch
  'pl': 'pl',    // Polish
  'pt': 'pt',    // Portuguese
  'ru': 'ru',    // Russian
  'sl': 'sl',    // Slovenian
  'tr': 'tr',    // Turkish
  'uk': 'uk',    // Ukrainian
  'zh': 'zh'     // Chinese
};

// All supported locale files
const localeFiles = Object.keys(LOCALE_CODE_MAP).map(code => `${code}.json`);

/**
 * Translate text using LibreTranslate
 */
async function translateText(text, targetLang) {
  const sourceLang = 'en';
  const langCode = LOCALE_CODE_MAP[targetLang];

  if (!langCode) {
    throw new Error(`Unsupported language code: ${targetLang}`);
  }

  // Don't translate if it's a variable placeholder
  if (text.startsWith('{{') && text.endsWith('}}')) {
    return text;
  }

  // Skip translation for very short strings or special formats
  if (text.length < 2) {
    return text;
  }

  try {
    const body = {
      q: text,
      source: sourceLang,
      target: langCode,
      format: 'text'
    };

    const headers = {
      'Content-Type': 'application/json'
    };

    // Add API key if provided
    if (LIBRE_TRANSLATE_API_KEY) {
      headers['Authorization'] = `Bearer ${LIBRE_TRANSLATE_API_KEY}`;
    }

    const response = await fetch(`${LIBRE_TRANSLATE_API}/translate`, {
      method: 'POST',
      headers,
      body: JSON.stringify(body)
    });

    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Translation API error: ${response.status} - ${errorText}`);
    }

    const data = await response.json();

    // Preserve placeholders in the translation
    return preservePlaceholders(text, data.translatedText);
  } catch (error) {
    console.warn(`    ⚠️  Translation failed for "${text}": ${error.message}`);
    // Return original text with a marker if translation fails
    return `TODO: ${text}`;
  }
}

/**
 * Preserve i18next placeholders in translated text
 */
function preservePlaceholders(original, translated) {
  // Extract all placeholders from original (e.g., {{count}}, {{boardName}})
  const placeholderRegex = /\{\{([^}]+)\}\}/g;
  const placeholders = original.match(placeholderRegex) || [];

  // If no placeholders, return translated as-is
  if (placeholders.length === 0) {
    return translated;
  }

  // Replace placeholders back in translated text
  let result = translated;
  placeholders.forEach(placeholder => {
    // Try to find if placeholder was translated and replace it back
    const varName = placeholder.match(/\{\{([^}]+)\}\}/)[1];
    // Common patterns that might appear in translation
    const patterns = [
      `{{${varName}}}`,
      `{{ ${varName} }}`,
      `{${varName}}`,
      `{ ${varName} }`,
      `%{${varName}}`,
      `%{${varName}}`
    ];

    // Look for any variation and replace with correct format
    for (const pattern of patterns) {
      if (result.includes(pattern) && pattern !== placeholder) {
        result = result.replace(pattern, placeholder);
        break;
      }
    }
  });

  return result;
}

/**
 * Translate multiple texts in batch for better performance
 */
async function translateBatch(texts, targetLang) {
  const results = [];

  // Process in smaller batches to avoid rate limiting
  const batchSize = 5;
  for (let i = 0; i < texts.length; i += batchSize) {
    const batch = texts.slice(i, i + batchSize);
    const translations = await Promise.all(
      batch.map(text => translateText(text, targetLang))
    );
    results.push(...translations);

    // Small delay between batches to be respectful to the API
    if (i + batchSize < texts.length) {
      await new Promise(resolve => setTimeout(resolve, 500));
    }
  }

  return results;
}

/**
 * Recursively get all keys from an object using dot notation
 */
function getKeys(obj, prefix = '') {
  const keys = [];
  for (const [key, value] of Object.entries(obj)) {
    const fullKey = prefix ? `${prefix}.${key}` : key;
    if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
      keys.push(...getKeys(value, fullKey));
    } else {
      keys.push(fullKey);
    }
  }
  return keys;
}

/**
 * Collect all missing translations with their paths
 */
function collectMissingTranslations(source, target, path = '', missing = []) {
  for (const [key, sourceValue] of Object.entries(source)) {
    const fullKey = path ? `${path}.${key}` : key;

    if (!(key in target)) {
      // Key is missing in target
      if (typeof sourceValue === 'object' && sourceValue !== null && !Array.isArray(sourceValue)) {
        collectMissingTranslations(sourceValue, {}, fullKey, missing);
      } else {
        missing.push({ path: fullKey, value: sourceValue });
      }
    } else if (typeof sourceValue === 'object' && sourceValue !== null && !Array.isArray(sourceValue)) {
      // Recurse into nested objects
      collectMissingTranslations(sourceValue, target[key], fullKey, missing);
    }
  }
  return missing;
}

/**
 * Set a value in a nested object using dot notation
 */
function setByPath(obj, path, value) {
  const keys = path.split('.');
  let current = obj;
  for (let i = 0; i < keys.length - 1; i++) {
    if (!(keys[i] in current)) {
      current[keys[i]] = {};
    }
    current = current[keys[i]];
  }
  current[keys[keys.length - 1]] = value;
}

/**
 * Deep clone an object
 */
function deepClone(obj) {
  return JSON.parse(JSON.stringify(obj));
}

console.log('🔍 Syncing translation files with en.json (source of truth)\n');
console.log(`🌐 Using LibreTranslate API: ${LIBRE_TRANSLATE_API}`);

if (LIBRE_TRANSLATE_API_KEY) {
  console.log('✅ API key is configured\n');
} else {
  console.log('⚠️  No API key configured - using public endpoint (rate limited)\n');
}

// Read the source English file
const sourceContent = fs.readFileSync(sourceFile, 'utf-8');
const sourceData = JSON.parse(sourceContent);
const sourceKeys = getKeys(sourceData);
console.log(`✅ Source file has ${sourceKeys.length} keys\n`);

let hasAnyChanges = false;

// Process each locale file
for (const localeFile of localeFiles) {
  const localePath = path.join(localesDir, localeFile);
  const localeName = localeFile.replace('.json', '');

  console.log(`📝 Processing ${localeName}...`);

  // Read locale file
  const localeContent = fs.readFileSync(localePath, 'utf-8');
  const localeData = JSON.parse(localeContent);
  const localeKeys = getKeys(localeData);

  // Find missing keys
  const missingTranslations = collectMissingTranslations(sourceData, localeData);

  if (missingTranslations.length === 0) {
    console.log(`  ✅ ${localeName} is up to date (${localeKeys.length} keys)\n`);
    continue;
  }

  console.log(`  ⚠️  Found ${missingTranslations.length} missing keys`);

  // Translate missing keys
  const textsToTranslate = missingTranslations.map(t => t.value);
  console.log(`  🌐 Translating ${textsToTranslate.length} strings...`);

  const translatedTexts = await translateBatch(textsToTranslate, localeName);

  // Create updated locale data
  const updatedLocaleData = deepClone(localeData);

  // Add translated keys
  missingTranslations.forEach((item, index) => {
    setByPath(updatedLocaleData, item.path, translatedTexts[index]);
  });

  // Write updated file
  fs.writeFileSync(localePath, JSON.stringify(updatedLocaleData, null, 2) + '\n');
  console.log(`  ✅ Updated ${localeName} with ${missingTranslations.length} new keys\n`);
  hasAnyChanges = true;
}

if (hasAnyChanges) {
  console.log('✨ Translation files updated successfully!');
  console.log('\n📋 Summary:');
  console.log('  - New keys have been automatically translated');
  console.log('  - Please review translations for accuracy');
  console.log('  - Some translations may need manual adjustment for context');
  process.exit(1); // Exit with error to indicate changes were made
} else {
  console.log('✅ All translation files are up to date!');
  process.exit(0);
}
