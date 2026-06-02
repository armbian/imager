import { useState, useEffect, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Monitor, Moon, Search, Sun } from 'lucide-react';
import { load } from '@tauri-apps/plugin-store';
import { useTheme } from '../../contexts/ThemeContext';
import { changeLanguage as i18nChangeLanguage, getCurrentLanguage } from '../../i18n';
import { SUPPORTED_LANGUAGES, flagUrl } from '../../config/i18n';

/** Theme option metadata for the segmented theme selector */
interface ThemeOption {
  /** Theme identifier passed to setTheme */
  value: 'light' | 'dark' | 'auto';
  /** Lucide icon component for the option */
  Icon: typeof Sun;
  /** i18n key for the option label */
  labelKey: string;
}

/** Available theme options rendered as segmented cards */
const THEME_OPTIONS: ThemeOption[] = [
  { value: 'light', Icon: Sun, labelKey: 'settings.themeLight' },
  { value: 'dark', Icon: Moon, labelKey: 'settings.themeDark' },
  { value: 'auto', Icon: Monitor, labelKey: 'settings.themeAuto' },
];

/** Appearance settings: theme cards + searchable language grid. Defaults language to "auto"
 * when none saved; flags are bundled twemoji SVGs (no runtime CDN or emoji-font dependency). */
export function AppearanceSection() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
  const [currentLanguage, setCurrentLanguage] = useState<string>(getCurrentLanguage());
  const [initialized, setInitialized] = useState(false);
  const [search, setSearch] = useState('');

  // Default to "auto" when no language is explicitly saved
  useEffect(() => {
    const checkAutoLanguage = async () => {
      try {
        const store = await load('settings.json', { autoSave: true, defaults: {} });
        const savedLanguage = await store.get<string>('language');
        if (!savedLanguage) {
          setCurrentLanguage('auto');
        }
      } catch (error) {
        console.error('Failed to check language mode:', error);
      } finally {
        setInitialized(true);
      }
    };
    checkAutoLanguage();
  }, []);

  /** Apply the selected UI language immediately and track it in state.
   * @param langCode - language code to activate (or "auto" for system locale) */
  const handleLanguageChange = async (langCode: string) => {
    try {
      await i18nChangeLanguage(langCode);
      setCurrentLanguage(langCode);
    } catch (error) {
      console.error('Failed to change language:', error);
    }
  };

  // Filter languages by name (case-insensitive); the "auto" entry is always shown
  const filteredLanguages = useMemo(() => {
    const query = search.trim().toLowerCase();
    if (!query) return SUPPORTED_LANGUAGES;
    return SUPPORTED_LANGUAGES.filter(
      (lang) => lang.code === 'auto' || lang.name.toLowerCase().includes(query)
    );
  }, [search]);

  if (!initialized) return null;

  return (
    <div className="settings-section">
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.chooseTheme')}</h4>

        <div className="theme-seg" role="group" aria-label={t('settings.chooseTheme')}>
          {THEME_OPTIONS.map(({ value, Icon, labelKey }) => (
            <button
              key={value}
              type="button"
              className={`theme-seg__card ${theme === value ? 'theme-seg__card--active' : ''}`}
              aria-pressed={theme === value}
              onClick={() => setTheme(value)}
            >
              <Icon size={28} className="theme-seg__icon" />
              <span className="theme-seg__label">{t(labelKey)}</span>
            </button>
          ))}
        </div>
      </div>

      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.chooseLanguage')}</h4>

        <div className="settings-search">
          <Search size={16} className="settings-search__icon" />
          <input
            type="text"
            className="settings-search__input"
            placeholder={t('settings.searchLanguage')}
            aria-label={t('settings.searchLanguage')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>

        <div className="settings-langgrid">
          {filteredLanguages.length === 0 ? (
            <div className="no-results">
              {t('settings.noLanguagesFound', { defaultValue: 'No languages found' })}
            </div>
          ) : (
            filteredLanguages.map((lang) => {
              const isActive = currentLanguage === lang.code;
              return (
                <button
                  key={lang.code}
                  type="button"
                  className={`settings-langgrid__item ${isActive ? 'settings-langgrid__item--active' : ''}`}
                  aria-current={isActive ? 'true' : undefined}
                  onClick={() => handleLanguageChange(lang.code)}
                >
                  {isActive && <Check size={15} className="settings-langgrid__check" strokeWidth={3} />}
                  <span className="settings-langgrid__flag">
                    {flagUrl(lang.code) ? (
                      <img src={flagUrl(lang.code)} className="emoji" alt="" />
                    ) : (
                      lang.flag
                    )}
                  </span>
                  <span className="settings-langgrid__label">
                    {lang.name || t('settings.languageAuto')}
                  </span>
                </button>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
