# Translation Sync Scripts

This directory contains automation scripts for managing i18n translations.

## sync-locales.js

Automatically syncs all translation files with `src/locales/en.json` (the source of truth) and translates missing keys using AI.

### Features

- **Automatic Detection**: Finds missing keys in all locale files
- **AI Translation**: Uses OpenAI API to automatically translate new keys with high quality
- **Context-Aware**: Provides section/key context to the AI for better translations
- **Placeholder Preservation**: Maintains i18next placeholders like `{{count}}` and `{{boardName}}`
- **Adaptive Rate Limiting**: Automatically adjusts based on model and payment tier
- **Error Handling**: Falls back to `TODO:` prefix if translation fails
- **Smart Prompts**: Uses specialized prompts for technical UI translation

### Usage

#### Local Development

```bash
# Basic usage (requires OpenAI API key)
export OPENAI_API_KEY=sk-...
node scripts/sync-locales.js

# With custom model (default: gpt-4o-mini)
OPENAI_MODEL=gpt-4o node scripts/sync-locales.js

# With custom OpenAI-compatible API endpoint
OPENAI_API=https://api.openai.com/v1 node scripts/sync-locales.js

# For paid tier (much faster - 50-100x speedup)
OPENAI_TIER=paid node scripts/sync-locales.js

# Retry failed translations (keys marked with TODO:)
RETRY_FAILED=true node scripts/sync-locales.js
```

#### GitHub Actions

The workflow runs automatically:
- **Daily** at 00:00 UTC
- **On push** to the branch
- **On manual trigger** via workflow_dispatch

### Configuration

#### Environment Variables

| Variable | Description | Default | Required |
|----------|-------------|---------|----------|
| `OPENAI_API_KEY` | OpenAI API key | - | Yes |
| `OPENAI_MODEL` | Model to use for translation | `gpt-4o-mini` | No |
| `OPENAI_API` | API endpoint URL | `https://api.openai.com/v1` | No |
| `OPENAI_TIER` | Account tier for rate limits | `free` | No |
| `RETRY_FAILED` | Retry keys marked with `TODO:` | `false` | No |

#### GitHub Secrets/Variables

To configure the GitHub Action:

1. **Required - Add API key**:
   ```bash
   gh secret set OPENAI_API_KEY
   ```

2. **Optional - Custom model** (for cost/quality tuning):
   ```bash
   gh variable set OPENAI_MODEL --value "gpt-4o-mini"
   ```

3. **Optional - Custom endpoint** (for OpenAI-compatible APIs):
   ```bash
   gh variable set OPENAI_API --value "https://api.openai.com/v1"
   ```

4. **Optional - Set account tier** (for faster translations with paid account):
   ```bash
   gh variable set OPENAI_TIER --value "paid"
   ```

5. **Optional - Retry failed translations** (re-attempt keys marked with `TODO:`):
   ```bash
   gh variable set RETRY_FAILED --value "true"
   ```

### OpenAI Setup

#### Getting an API Key

1. Visit [platform.openai.com](https://platform.openai.com/)
2. Sign up or log in
3. Navigate to API Keys section
4. Create a new API key
5. Add it to your environment or GitHub secrets

#### Account Tier & Rate Limits

The script automatically adjusts speed based on your account tier:

| Tier | RPM Limit | Batch Size | Delay | 100 Keys Time |
|------|-----------|------------|-------|---------------|
| **Free** | 3/min | 1 | 21s | ~35 min |
| **Paid (Tier 1-2)** | 200/min | 50 | 300ms | ~1 min |
| **Paid (Tier 3-5)** | 500/min | 50 | 120ms | ~30 sec |

To use paid tier rates:
1. Add a payment method to your OpenAI account (even $5 works)
2. Set `OPENAI_TIER=paid` environment variable

**Recommendation**: With just a $5 balance, you get Tier 1-2 rates which are **65x faster** than free tier.

#### Choosing a Model

| Model | Cost | Quality | Speed | Best For |
|-------|------|---------|-------|----------|
| `gpt-4o-mini` | Low | High | Fast | Most translations (default) |
| `gpt-4o` | Medium | Very High | Fast | Complex UI text |
| `gpt-3.5-turbo` | Very Low | Medium | Very Fast | Simple translations |

**Recommendation**: Start with `gpt-4o-mini` for the best balance of cost, quality, and speed.

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
| `zh` | Chinese (Simplified) |

### Output

The script will:
1. ✅ Show which keys are missing for each language
2. 🤖 Translate missing keys with OpenAI
3. 📊 Show translation statistics (success/failure)
4. ⚠️  Warn about any translation failures
5. 💾 Update locale files with translated content
6. 🔍 Exit with code 1 if changes were made (useful for CI/CD)

### Example Output

```
🔍 Syncing translation files with en.json (source of truth)

🤖 Using OpenAI API: https://api.openai.com/v1
📦 Model: gpt-4o-mini
✅ API key is configured

✅ Source file has 93 keys

📝 Processing de (German)...
  ✅ de is up to date (93 keys)

📝 Processing hr (Croatian)...
  ⚠️  Found 64 missing keys
  🤖 Translating 64 strings with OpenAI...
  ✅ Updated hr with 64 new keys

✨ Translation files updated successfully!

📊 Summary:
  - Total translated: 64 keys
  - Please review translations for accuracy and context
```

### AI Translation Features

The script uses specialized prompts to ensure:

1. **Context Awareness**: Provides section/key context for each translation
2. **Technical Terminology**: Knows when to keep terms like "Flash", "SD card", "USB" in English
3. **Placeholder Preservation**: Maintains `{{variables}}` exactly as they appear
4. **UI Appropriateness**: Uses concise, natural text for buttons and labels
5. **Plural Forms**: Handles i18next plural suffixes (_one, _other) correctly
6. **Consistent Tone**: Maintains formal but friendly tone throughout

### Best Practices

1. **Review Translations**: AI translations are excellent but may need context-specific adjustments
2. **Test in App**: Always test translations in the actual application
3. **Handle Plurals**: The script preserves `_one` and `_other` suffixes for plural forms
4. **Check Placeholders**: Verify that `{{variables}}` are correctly preserved
5. **Cultural Nuances**: Review translations for cultural appropriateness
6. **Cost Management**: Use `gpt-4o-mini` for best cost/quality balance

### Troubleshooting

#### API Key Not Found

```
❌ OPENAI_API_KEY is not set!
```

**Solution**: Set the environment variable or GitHub secret:
```bash
export OPENAI_API_KEY=sk-...
```

#### Translation Failures

If some translations fail with `TODO:`:
- Check your API key has sufficient credits
- Verify API endpoint is accessible
- Check rate limits (especially for larger translation batches

#### Poor Quality Translations

If translations seem off:
- The AI might lack specific UI context
- Try a more capable model like `gpt-4o`
- Manually edit the JSON files to fix issues
- Consider the translation context provided in the prompt

#### Cost Concerns

For cost optimization:
- Use `gpt-4o-mini` (very cost-effective)
- Run the script less frequently
- Review and merge translations in batches
- Consider caching previous translations

### Cost Estimation

Approximate costs for translating missing keys (using `gpt-4o-mini`):

- **10 keys**: ~$0.00001
- **50 keys**: ~$0.00005
- **100 keys**: ~$0.0001

*Costs vary based on text length and model used.*
