import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Settings, Terminal } from 'lucide-react';
import { AppearanceSection } from './AppearanceSection';
import { LanguageSection } from './LanguageSection';
import { GeneralSection } from './GeneralSection';
import { AboutSection } from './AboutSection';
import { AdvancedSection } from './AdvancedSection';

type SettingsView = 'general' | 'appearance' | 'language' | 'advanced' | 'about';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Settings modal with sidebar navigation
 *
 * Features a left sidebar with category options and a right content area.
 * Matches the classic settings UI pattern shown in the reference image.
 */
export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsView>('general');

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="settings-modal" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="settings-modal-header">
          <h2>{t('settings.title')}</h2>
          <button className="modal-close" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        {/* Sidebar + Content */}
        <div className="settings-modal-body">
          {/* Sidebar */}
          <div className="settings-sidebar">
            <button
              className={`settings-sidebar-item ${activeSection === 'general' ? 'active' : ''}`}
              onClick={() => setActiveSection('general')}
            >
              <Settings size={20} />
              <span>{t('settings.general')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'appearance' ? 'active' : ''}`}
              onClick={() => setActiveSection('appearance')}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="5" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
              </svg>
              <span>{t('settings.theme')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'language' ? 'active' : ''}`}
              onClick={() => setActiveSection('language')}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2"/>
                <path d="M2 12h20M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" stroke="currentColor" strokeWidth="2"/>
              </svg>
              <span>{t('settings.language')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'advanced' ? 'active' : ''}`}
              onClick={() => setActiveSection('advanced')}
            >
              <Terminal size={20} />
              <span>{t('settings.advancedCategory')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'about' ? 'active' : ''}`}
              onClick={() => setActiveSection('about')}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="2"/>
                <path d="M12 16v-4M12 8h.01" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
              </svg>
              <span>{t('settings.appInfo')}</span>
            </button>
          </div>

          {/* Content Area */}
          <div className="settings-content">
            {activeSection === 'general' && <GeneralSection />}
            {activeSection === 'appearance' && <AppearanceSection />}
            {activeSection === 'language' && <LanguageSection />}
            {activeSection === 'advanced' && <AdvancedSection />}
            {activeSection === 'about' && <AboutSection />}
          </div>
        </div>
      </div>
    </div>
  );
}
