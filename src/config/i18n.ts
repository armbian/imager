// Supported-language config. To add one: drop src/locales/{code}.json and add an entry below.

// Twemoji flag SVGs from the maintained @twemoji/svg package; Vite bundles only these
// (offline-safe, no runtime CDN, no assets in the repo).
import flagAuto from '@twemoji/svg/1f310.svg';
import flagEn from '@twemoji/svg/1f1ec-1f1e7.svg';
import flagDe from '@twemoji/svg/1f1e9-1f1ea.svg';
import flagEs from '@twemoji/svg/1f1ea-1f1f8.svg';
import flagFr from '@twemoji/svg/1f1eb-1f1f7.svg';
import flagHr from '@twemoji/svg/1f1ed-1f1f7.svg';
import flagIt from '@twemoji/svg/1f1ee-1f1f9.svg';
import flagJa from '@twemoji/svg/1f1ef-1f1f5.svg';
import flagKo from '@twemoji/svg/1f1f0-1f1f7.svg';
import flagNl from '@twemoji/svg/1f1f3-1f1f1.svg';
import flagPl from '@twemoji/svg/1f1f5-1f1f1.svg';
import flagPt from '@twemoji/svg/1f1f5-1f1f9.svg';
import flagPtBr from '@twemoji/svg/1f1e7-1f1f7.svg';
import flagRu from '@twemoji/svg/1f1f7-1f1fa.svg';
import flagSl from '@twemoji/svg/1f1f8-1f1ee.svg';
import flagSv from '@twemoji/svg/1f1f8-1f1ea.svg';
import flagTr from '@twemoji/svg/1f1f9-1f1f7.svg';
import flagUk from '@twemoji/svg/1f1fa-1f1e6.svg';
import flagZh from '@twemoji/svg/1f1e8-1f1f3.svg';

export interface LanguageMetadata {
  /** ISO 639-1 language code */
  code: string;
  /** Native language name (e.g., "Italiano" for Italian) */
  name: string;
  /** Flag emoji for visual identification */
  flag: string;
}

const LANGUAGES: LanguageMetadata[] = [
  { code: 'en', name: 'English', flag: '🇬🇧' },
  { code: 'de', name: 'Deutsch', flag: '🇩🇪' },
  { code: 'es', name: 'Español', flag: '🇪🇸' },
  { code: 'fr', name: 'Français', flag: '🇫🇷' },
  { code: 'hr', name: 'Hrvatski', flag: '🇭🇷' },
  { code: 'it', name: 'Italiano', flag: '🇮🇹' },
  { code: 'ja', name: '日本語', flag: '🇯🇵' },
  { code: 'ko', name: '한국어', flag: '🇰🇷' },
  { code: 'nl', name: 'Nederlands', flag: '🇳🇱' },
  { code: 'pl', name: 'Polski', flag: '🇵🇱' },
  { code: 'pt', name: 'Português', flag: '🇵🇹' },
  { code: 'pt-BR', name: 'Português do Brasil', flag: '🇧🇷' },
  { code: 'ru', name: 'Русский', flag: '🇷🇺' },
  { code: 'sl', name: 'Slovenščina', flag: '🇸🇮' },
  { code: 'sv', name: 'Svenska', flag: '🇸🇪' },
  { code: 'tr', name: 'Türkçe', flag: '🇹🇷' },
  { code: 'uk', name: 'Українська', flag: '🇺🇦' },
  { code: 'zh', name: '中文', flag: '🇨🇳' },
];

/** Auto language option; name is set dynamically in the UI via translation */
const AUTO_LANGUAGE: LanguageMetadata = {
  code: 'auto',
  name: '',  // Will be translated in UI
  flag: '🌐',
};

export const SUPPORTED_LANGUAGES: LanguageMetadata[] = [
  AUTO_LANGUAGE,
  ...LANGUAGES.sort((a, b) => a.name.localeCompare(b.name))
];

/** Get all supported language codes */
export function getSupportedLanguageCodes(): string[] {
  return SUPPORTED_LANGUAGES.map((lang) => lang.code);
}

/** Get the default language (English) */
export function getDefaultLanguage(): string {
  return 'en';
}

/** Extract the language code from a locale string (e.g. "en-US" -> "en") */
export function getLanguageFromLocale(locale: string): string {
  const lang = locale.split('-')[0].toLowerCase();
  return getSupportedLanguageCodes().includes(lang) ? lang : getDefaultLanguage();
}

/** Language code -> bundled twemoji flag SVG URL. */
const FLAG_URLS: Record<string, string> = {
  auto: flagAuto, en: flagEn, de: flagDe, es: flagEs, fr: flagFr, hr: flagHr,
  it: flagIt, ja: flagJa, ko: flagKo, nl: flagNl, pl: flagPl, pt: flagPt,
  'pt-BR': flagPtBr, ru: flagRu, sl: flagSl, sv: flagSv, tr: flagTr, uk: flagUk, zh: flagZh,
};

/** Resolve the bundled twemoji flag SVG URL for a language code. */
export function flagUrl(code: string): string | undefined {
  return FLAG_URLS[code];
}
