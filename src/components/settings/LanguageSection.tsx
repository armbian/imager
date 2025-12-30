import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import twemoji from 'twemoji';
import { load } from '@tauri-apps/plugin-store';
import { changeLanguage as i18nChangeLanguage, getCurrentLanguage } from '../../i18n';
import { SUPPORTED_LANGUAGES } from '../../config/i18n';

/**
 * Language settings section for sidebar layout
 *
 * Displays a list of languages with flag emojis.
 * Uses Twemoji for consistent flag rendering across platforms.
 */
export function LanguageSection() {
  const { t } = useTranslation();
  const [currentLanguage, setCurrentLanguage] = useState<string>(getCurrentLanguage());
  const languageListRef = useRef<HTMLDivElement>(null);

  /**
   * Check if language is set to auto on mount
   */
  useEffect(() => {
    const checkAutoLanguage = async () => {
      try {
        const store = await load('settings.json', { autoSave: true, defaults: {} });
        const savedLanguage = await store.get<string>('language');
        const autoMode = !savedLanguage;
        if (autoMode) {
          setCurrentLanguage('auto');
        }
      } catch (error) {
        console.error('Failed to check language mode:', error);
      }
    };
    checkAutoLanguage();
  }, []);

  /**
   * Parse flags with Twemoji when component mounts or language changes
   */
  useEffect(() => {
    if (languageListRef.current) {
      twemoji.parse(languageListRef.current);
    }
  }, [currentLanguage]);

  /**
   * Handle language change
   */
  const handleLanguageChange = async (langCode: string) => {
    try {
      await i18nChangeLanguage(langCode);
      setCurrentLanguage(langCode);
    } catch (error) {
      console.error('Failed to change language:', error);
    }
  };

  return (
    <div className="settings-section" ref={languageListRef}>
      <h3 className="settings-section-title">{t('settings.chooseLanguage')}</h3>

      <div className="settings-list">
        {SUPPORTED_LANGUAGES.map((lang) => (
          <div
            key={lang.code}
            className={`settings-list-item ${currentLanguage === lang.code ? 'active' : ''}`}
            onClick={() => handleLanguageChange(lang.code)}
          >
            <div className="settings-list-item-left">
              <span className="language-flag-emoji">{lang.flag}</span>
              <span className="settings-list-item-label">
                {lang.name || t('settings.languageAuto')}
              </span>
            </div>
            {currentLanguage === lang.code && (
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <polyline points="20 6 9 17 4 12" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
