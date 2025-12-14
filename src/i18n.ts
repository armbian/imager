import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';

import en from './locales/en.json';
import it from './locales/it.json';
import de from './locales/de.json';
import fr from './locales/fr.json';
import es from './locales/es.json';
import pt from './locales/pt.json';
import nl from './locales/nl.json';
import pl from './locales/pl.json';
import ru from './locales/ru.json';
import zh from './locales/zh.json';
import ja from './locales/ja.json';
import ko from './locales/ko.json';
import uk from './locales/uk.json';
import tr from './locales/tr.json';
import sl from './locales/sl.json';

const resources = {
  en: { translation: en },
  it: { translation: it },
  de: { translation: de },
  fr: { translation: fr },
  es: { translation: es },
  pt: { translation: pt },
  nl: { translation: nl },
  pl: { translation: pl },
  ru: { translation: ru },
  zh: { translation: zh },
  ja: { translation: ja },
  ko: { translation: ko },
  uk: { translation: uk },
  tr: { translation: tr },
  sl: { translation: sl },
};

// Supported languages
const supportedLanguages = ['en', 'it', 'de', 'fr', 'es', 'pt', 'nl', 'pl', 'ru', 'zh', 'ja', 'ko', 'uk', 'tr', 'sl'];

/**
 * Extract language code from locale string
 * e.g., "en-US" -> "en", "it-IT" -> "it"
 */
function getLanguageFromLocale(locale: string): string {
  const lang = locale.split('-')[0].toLowerCase();
  return supportedLanguages.includes(lang) ? lang : 'en';
}

/**
 * Initialize i18n with system locale detection
 */
export async function initI18n(): Promise<void> {
  let systemLocale = 'en-US';

  try {
    // Get system locale from Tauri backend
    systemLocale = await invoke<string>('get_system_locale');
  } catch (error) {
    console.warn('Failed to get system locale, using default:', error);
  }

  const language = getLanguageFromLocale(systemLocale);

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

export default i18n;
