import { useState, useEffect, useRef } from 'react';
import { useTranslation } from 'react-i18next';
import twemoji from 'twemoji';
import { load } from '@tauri-apps/plugin-store';
import { useTheme } from '../../contexts/ThemeContext';
import { changeLanguage as i18nChangeLanguage, getCurrentLanguage } from '../../i18n';
import { SUPPORTED_LANGUAGES } from '../../config/i18n';

/**
 * Appearance settings section combining theme and language selection
 *
 * Theme: Light, Dark, Auto clickable box cards with icons.
 * Language: List of 18 languages with Twemoji flag rendering.
 */
export function AppearanceSection() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();
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
    <div className="settings-section">
      {/* THEME Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">{t('settings.chooseTheme')}</h4>

        <div className="theme-boxes">
          <div
            className={`theme-box ${theme === 'light' ? 'active' : ''}`}
            onClick={() => setTheme('light')}
          >
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <circle cx="12" cy="12" r="5" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
              <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
            </svg>
            <span className="theme-box-label">{t('settings.themeLight')}</span>
          </div>

          <div
            className={`theme-box ${theme === 'dark' ? 'active' : ''}`}
            onClick={() => setTheme('dark')}
          >
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            <span className="theme-box-label">{t('settings.themeDark')}</span>
          </div>

          <div
            className={`theme-box ${theme === 'auto' ? 'active' : ''}`}
            onClick={() => setTheme('auto')}
          >
            <svg width="32" height="32" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
              <rect x="2" y="3" width="20" height="14" rx="2" ry="2" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              <line x1="8" y1="21" x2="16" y2="21" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
              <line x1="12" y1="17" x2="12" y2="21" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"/>
            </svg>
            <span className="theme-box-label">{t('settings.themeAuto')}</span>
          </div>
        </div>
      </div>

      {/* LANGUAGE Section */}
      <div className="settings-category" ref={languageListRef}>
        <h4 className="settings-category-title">{t('settings.chooseLanguage')}</h4>

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
    </div>
  );
}
