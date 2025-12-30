import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { load } from '@tauri-apps/plugin-store';
import { SUPPORTED_LANGUAGES, getLanguageFromLocale } from './config/i18n';

/**
 * Dynamically load all translation files
 *
 * Uses Vite's import.meta.glob to load all JSON files in locales directory.
 * This eliminates the need for static imports and automatically includes
 * any new language files added to the locales folder.
 */
const localeModules = import.meta.glob('./locales/*.json', { eager: true });

/**
 * Build resources object from dynamically loaded locale files
 *
 * Extracts the language code from the filename (e.g., './locales/en.json' -> 'en')
 * and creates the i18next-compatible resources structure.
 */
const resources = Object.entries(localeModules).reduce((acc, [path, module]) => {
  // Extract language code from path: './locales/en.json' -> 'en'
  const langCode = path.match(/\.\/locales\/(.+)\.json$/)?.[1];
  if (langCode && module) {
    acc[langCode] = { translation: module as Record<string, unknown> };
  }
  return acc;
}, {} as Record<string, { translation: Record<string, unknown> }>);

// Export supported language codes for use in other components
export const supportedLanguages = SUPPORTED_LANGUAGES.map((lang) => lang.code);

/**
 * Initialize i18n with saved language or system locale detection
 */
export async function initI18n(): Promise<void> {
  let language = 'en';

  try {
    // Try to load saved language first using Store plugin
    const store = await load('settings.json', { autoSave: true, defaults: {} });
    const savedLanguage = await store.get<string>('language');
    if (savedLanguage) {
      language = savedLanguage;
    }
  } catch {
    // If no saved language, detect from system locale
    try {
      const systemLocale = await invoke<string>('get_system_locale');
      language = getLanguageFromLocale(systemLocale);
    } catch (localeError) {
      console.warn('Failed to get system locale, using default:', localeError);
      language = 'en';
    }
  }

  await i18n
    .use(initReactI18next)
    .init({
      resources,
      lng: language,
      fallbackLng: 'en',
      interpolation: {
        escapeValue: false, // React already escapes values
      },
      react: {
        useSuspense: false, // Disable suspense for sync initialization
      },
    });
}

/**
 * Change the current language and persist to storage
 * @param lang - Language code to change to (e.g., 'en', 'it', 'auto')
 */
export async function changeLanguage(lang: string): Promise<void> {
  const store = await load('settings.json', { autoSave: true, defaults: {} });

  if (lang === 'auto') {
    // Remove saved language to enable auto-detection
    try {
      await store.delete('language');
    } catch (error) {
      console.error('Failed to delete language from storage:', error);
    }

    // Detect system locale and change to it
    try {
      const systemLocale = await invoke<string>('get_system_locale');
      const detectedLang = getLanguageFromLocale(systemLocale);
      await i18n.changeLanguage(detectedLang);
    } catch (localeError) {
      console.warn('Failed to get system locale, using default:', localeError);
      await i18n.changeLanguage('en');
    }
  } else {
    // Change language in i18next
    await i18n.changeLanguage(lang);

    // Persist to storage using Store plugin
    try {
      await store.set('language', lang);
    } catch (error) {
      console.error('Failed to save language to storage:', error);
    }
  }
}

/**
 * Get the current language
 */
export function getCurrentLanguage(): string {
  return i18n.language;
}

/**
 * Check if a language is supported
 */
export function isLanguageSupported(lang: string): boolean {
  return supportedLanguages.includes(lang);
}

export default i18n;
