# Translation Sync Scripts

This directory contains automation scripts for managing i18n translations.

## sync-locales.js

Automatically syncs all translation files with `src/locales/en.json` (the source of truth) and translates missing keys using AI.

### Features

- **Automatic Detection**: Finds missing keys in all locale files
- **AI Translation**: Uses LibreTranslate to automatically translate new keys
- **Placeholder Preservation**: Maintains i18next placeholders like `{{count}}` and `{{boardName}}`
- **Batch Processing**: Translates in batches with rate limiting to be respectful to the API
- **Error Handling**: Falls back to `TODO:` prefix if translation fails

### Usage

#### Local Development

```bash
# Basic usage (uses public LibreTranslate endpoint - rate limited)
node scripts/sync-locales.js

# With custom LibreTranslate instance
LIBRE_TRANSLATE_API=https://your-instance.com node scripts/sync-locales.js

# With API key (for higher rate limits)
LIBRE_TRANSLATE_API_KEY=your-api-key node scripts/sync-locales.js
```

#### GitHub Actions

The workflow runs automatically:
- **Daily** at 00:00 UTC
- **On manual trigger** via workflow_dispatch

### Configuration

#### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `LIBRE_TRANSLATE_API` | LibreTranslate API endpoint | `https://libretranslate.com` | No |
| `LIBRE_TRANSLATE_API_KEY` | API key for authenticated requests | - | No |

#### GitHub Secrets/Variables

To configure the GitHub Action:

1. **Optional - Add API key** (recommended for higher rate limits):
   ```bash
   gh secret set LIBRE_TRANSLATE_API_KEY
   ```

2. **Optional - Custom endpoint**:
   ```bash
   gh variable set LIBRE_TRANSLATE_API --value "https://your-instance.com"
   ```

### LibreTranslate Setup

#### Option 1: Use Public Endpoint

The script works out-of-the-box using `https://libretranslate.com` with rate limitations.

#### Option 2: Get an API Key

1. Visit [LibreTranslate Cloud](https://libretranslate.com/)
2. Sign up for an account
3. Generate an API key
4. Add it to your GitHub secrets or use locally

#### Option 3: Self-Hosted Instance

Deploy your own LibreTranslate instance for better performance and privacy:

```bash
docker run -ti --rm -p 5000:5000 libretranslate/libretranslate
```

Then configure:
```bash
export LIBRE_TRANSLATE_API=http://localhost:5000
```

### Supported Languages

| Code | Language |
|------|----------|
| `de` | German |
| `es` | Spanish |
| `fr` | French |
| `it` | Italian |
| `ja` | Japanese |
| `ko` | Korean |
| `nl` | Dutch |
| `pl` | Polish |
| `pt` | Portuguese |
| `ru` | Russian |
| `sl` | Slovenian |
| `tr` | Turkish |
| `uk` | Ukrainian |
| `zh` | Chinese |

### Output

The script will:
1. ✅ Show which keys are missing for each language
2. 🌐 Translate missing keys automatically
3. ⚠️  Warn about any translation failures
4. 💾 Update locale files with translated content
5. 🔍 Exit with code 1 if changes were made (useful for CI/CD)

### Example Output

```
🔍 Syncing translation files with en.json (source of truth)

🌐 Using LibreTranslate API: https://libretranslate.com
✅ API key is configured

✅ Source file has 245 keys

📝 Processing de...
  ⚠️  Found 3 missing keys
  🌐 Translating 3 strings...
  ✅ Updated de with 3 new keys

📝 Processing es...
  ✅ es is up to date (245 keys)

...

✨ Translation files updated successfully!

📋 Summary:
  - New keys have been automatically translated
  - Please review translations for accuracy
  - Some translations may need manual adjustment for context
```

### Best Practices

1. **Review Translations**: AI translations are good starting points but may need context-specific adjustments
2. **Test in App**: Always test translations in the actual application
3. **Handle Plurals**: The script preserves `_one` and `_other` suffixes for plural forms
4. **Check Placeholders**: Verify that `{{variables}}` are correctly preserved
5. **Cultural Nuances**: Review translations for cultural appropriateness

### Troubleshooting

#### Rate Limiting

If you see rate limit errors:
- Get an API key from LibreTranslate
- Or deploy your own instance

#### Translation Quality

If translations seem off:
- The translation might lack UI context
- Manually edit the JSON files
- Consider professional translation for key user-facing text

#### Placeholders Broken

If placeholders like `{{count}}` are malformed:
- The script attempts to preserve them
- Edit the locale file manually to fix format
- Report the issue if it happens consistently
