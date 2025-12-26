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

// OpenAI configuration
const OPENAI_API_KEY = process.env.OPENAI_API_KEY || '';
const OPENAI_MODEL = process.env.OPENAI_MODEL || 'gpt-4o-mini';
const OPENAI_API = process.env.OPENAI_API || 'https://api.openai.com/v1';

// Language names for better context in translation
const LANGUAGE_NAMES = {
  'de': 'German',
  'es': 'Spanish',
  'fr': 'French',
  'hr': 'Croatian',
  'it': 'Italian',
  'ja': 'Japanese',
  'ko': 'Korean',
  'nl': 'Dutch',
  'pl': 'Polish',
  'pt': 'Portuguese',
  'ru': 'Russian',
  'sl': 'Slovenian',
  'tr': 'Turkish',
  'uk': 'Ukrainian',
  'zh': 'Chinese (Simplified)'
};

// All supported locale files
const localeFiles = Object.keys(LANGUAGE_NAMES).map(code => `${code}.json`);

/**
 * Translate text using OpenAI API
 */
async function translateText(text, targetLang, context = '') {
  if (!OPENAI_API_KEY) {
    throw new Error('OPENAI_API_KEY is not set');
  }

  // Don't translate if it's a variable placeholder
  if (text.startsWith('{{') && text.endsWith('}}')) {
    return text;
  }

  // Skip translation for very short strings or special formats
  if (text.length < 2) {
    return text;
  }

  // Extract placeholders to preserve them
  const placeholderRegex = /\{\{([^}]+)\}\}/g;
  const placeholders = text.match(placeholderRegex) || [];

  try {
    const systemPrompt = `You are a professional translator for a software application called "Armbian Imager" - a tool for flashing operating system images to SD cards and USB drives.

Translate the given text to ${LANGUAGE_NAMES[targetLang]}.

Important rules:
1. Keep technical terms in English when appropriate (e.g., "SD card", "USB", "Flash", "Board", "Image")
2. Preserve ALL placeholders exactly as they appear (e.g., {{count}}, {{boardName}}, {{step}})
3. Use natural, concise UI text appropriate for buttons and labels
4. Maintain formal but friendly tone
5. For plural forms (text ending in _one or _other), translate appropriately for the grammatical number
6. Keep keyboard shortcuts and hotkeys in English
7. Only return the translated text, no explanations

${context ? `Context: ${context}` : ''}`;

    const response = await fetch(`${OPENAI_API}/chat/completions`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${OPENAI_API_KEY}`
      },
      body: JSON.stringify({
        model: OPENAI_MODEL,
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: text }
        ],
        temperature: 0.3,
        max_tokens: 500
      })
    });

    if (!response.ok) {
      const errorData = await response.json().catch(() => ({}));
      throw new Error(`OpenAI API error: ${response.status} - ${JSON.stringify(errorData)}`);
    }

    const data = await response.json();
    const translatedText = data.choices[0]?.message?.content || text;

    // Ensure placeholders are preserved
    return preservePlaceholders(text, translatedText);
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
    const varName = placeholder.match(/\{\{([^}]+)\}\}/)[1];

    // Look for variations and replace with correct format
    const patterns = [
      `{{${varName}}}`,
      `{{ ${varName} }}`,
      `{${varName}}`,
      `{ ${varName} }`,
      `%{${varName}}`,
      `%{ ${varName} }`
    ];

    for (const pattern of patterns) {
      if (result.includes(pattern) && pattern !== placeholder) {
        result = result.replaceAll(pattern, placeholder);
        break;
      }
    }

    // If placeholder completely missing, add it back
    if (!result.includes(placeholder)) {
      // Try to find where it should go (heuristic)
      const originalWithoutPlaceholder = original.replace(placeholder, '');
      if (translated.includes(originalWithoutPlaceholder)) {
        result = result.replace(originalWithoutPlaceholder, original);
      }
    }
  });

  return result;
}

/**
 * Translate multiple texts in batch for better performance
 */
async function translateBatch(texts, targetLang, contexts = []) {
  const results = [];

  // Process in smaller batches to avoid rate limiting
  const batchSize = 10;
  for (let i = 0; i < texts.length; i += batchSize) {
    const batch = texts.slice(i, i + batchSize);
    const batchContexts = contexts.slice(i, i + batchSize);

    const translations = await Promise.all(
      batch.map((text, idx) => translateText(text, targetLang, batchContexts[idx] || ''))
    );
    results.push(...translations);

    // Small delay between batches to be respectful to the API
    if (i + batchSize < texts.length) {
      await new Promise(resolve => setTimeout(resolve, 1000));
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
        // Provide context for better translations
        const context = `Section: ${path}, Key: ${key}`;
        missing.push({ path: fullKey, value: sourceValue, context });
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
console.log(`🤖 Using OpenAI API: ${OPENAI_API}`);
console.log(`📦 Model: ${OPENAI_MODEL}`);

if (!OPENAI_API_KEY) {
  console.error('❌ OPENAI_API_KEY is not set!');
  console.log('   Set it with: export OPENAI_API_KEY=your-key-here\n');
  process.exit(1);
}

console.log('✅ API key is configured\n');

// Read the source English file
const sourceContent = fs.readFileSync(sourceFile, 'utf-8');
const sourceData = JSON.parse(sourceContent);
const sourceKeys = getKeys(sourceData);
console.log(`✅ Source file has ${sourceKeys.length} keys\n`);

let hasAnyChanges = false;
let totalTranslated = 0;
let totalFailed = 0;

// Process each locale file
for (const localeFile of localeFiles) {
  const localePath = path.join(localesDir, localeFile);
  const localeName = localeFile.replace('.json', '');

  console.log(`📝 Processing ${localeName} (${LANGUAGE_NAMES[localeName]})...`);

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
  const contexts = missingTranslations.map(t => t.context);
  console.log(`  🤖 Translating ${textsToTranslate.length} strings with OpenAI...`);

  const translatedTexts = await translateBatch(textsToTranslate, localeName, contexts);

  // Count successes and failures
  const failedCount = translatedTexts.filter(t => t.startsWith('TODO:')).length;
  totalTranslated += translatedTexts.length - failedCount;
  totalFailed += failedCount;

  // Create updated locale data
  const updatedLocaleData = deepClone(localeData);

  // Add translated keys
  missingTranslations.forEach((item, index) => {
    setByPath(updatedLocaleData, item.path, translatedTexts[index]);
  });

  // Write updated file
  fs.writeFileSync(localePath, JSON.stringify(updatedLocaleData, null, 2) + '\n');

  if (failedCount > 0) {
    console.log(`  ⚠️  Updated ${localeName}: ${translatedTexts.length - failedCount} translated, ${failedCount} failed\n`);
  } else {
    console.log(`  ✅ Updated ${localeName} with ${translatedTexts.length} new keys\n`);
  }
  hasAnyChanges = true;
}

if (hasAnyChanges) {
  console.log('✨ Translation files updated successfully!');
  console.log('\n📊 Summary:');
  console.log(`  - Total translated: ${totalTranslated} keys`);
  if (totalFailed > 0) {
    console.log(`  - Total failed: ${totalFailed} keys (marked with TODO:)`);
  }
  console.log('  - Please review translations for accuracy and context');
  process.exit(1); // Exit with error to indicate changes were made
} else {
  console.log('✅ All translation files are up to date!');
  process.exit(0);
}
