// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com
// Copyright (c) 2025 Igor Pecovnik, igor@armbian.com

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import { getLanguageFromLocale, getDefaultLanguage, AUTO_LANGUAGE_CODE } from './config/i18n';
import { getLanguage, setLanguage, clearLanguage, loadSettingsStore } from './hooks/useSettings';

// Load every locale JSON via Vite glob so new languages are picked up automatically
const localeModules = import.meta.glob('./locales/*.json', { eager: true });

// Build the i18next resources map keyed by language code
const resources = Object.entries(localeModules).reduce((acc, [path, module]) => {
  // './locales/en.json' -> 'en'
  const langCode = path.match(/\.\/locales\/(.+)\.json$/)?.[1];
  if (langCode && module) {
    acc[langCode] = { translation: module as Record<string, unknown> };
  }
  return acc;
}, {} as Record<string, { translation: Record<string, unknown> }>);

/** Initialize i18n using the saved language, falling back to system locale detection */
export async function initI18n(): Promise<void> {
  let language = getDefaultLanguage();

  try {
    const savedLanguage = await getLanguage();
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
      language = getDefaultLanguage();
    }
  }

  await i18n
    .use(initReactI18next)
    .init({
      resources,
      lng: language,
      fallbackLng: getDefaultLanguage(),
      interpolation: {
        escapeValue: false, // React already escapes values
      },
      react: {
        useSuspense: false, // Disable suspense for sync initialization
      },
    });
}

/** Change the active language (e.g. 'en', 'it', 'auto') and persist it */
export async function changeLanguage(lang: string): Promise<void> {
  // An unavailable store rejects before the language switches, so the caller can report it
  await loadSettingsStore();

  if (lang === AUTO_LANGUAGE_CODE) {
    // Remove saved language to enable auto-detection
    try {
      await clearLanguage();
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
      await i18n.changeLanguage(getDefaultLanguage());
    }
  } else {
    // Change language in i18next
    await i18n.changeLanguage(lang);

    try {
      await setLanguage(lang);
    } catch (error) {
      console.error('Failed to save language to storage:', error);
    }
  }
}

/** Get the current language */
export function getCurrentLanguage(): string {
  return i18n.language;
}

export default i18n;
