import { useTranslation } from 'react-i18next';
import { useTheme } from '../../contexts/ThemeContext';

/**
 * Appearance settings section for sidebar layout
 *
 * Allows users to select between Light, Dark, and Auto themes.
 * Uses clickable box cards with icons.
 */
export function AppearanceSection() {
  const { t } = useTranslation();
  const { theme, setTheme } = useTheme();

  return (
    <div className="settings-section">
      <h3 className="settings-section-title">{t('settings.chooseTheme')}</h3>

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
  );
}
