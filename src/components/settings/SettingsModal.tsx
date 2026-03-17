import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Settings, Terminal, HardDrive } from 'lucide-react';
import { AppearanceSection } from './AppearanceSection';
import { PreferencesSection } from './PreferencesSection';
import { StorageSection } from './StorageSection';
import { DeveloperSection } from './DeveloperSection';
import { AboutSection } from './AboutSection';

type SettingsView = 'appearance' | 'preferences' | 'storage' | 'developer' | 'about';

interface SettingsModalProps {
  isOpen: boolean;
  onClose: () => void;
}

/**
 * Settings modal with sidebar navigation
 *
 * Features a left sidebar with category options and a right content area.
 * Tabs: Appearance, Preferences, Storage, Developer, About.
 */
export function SettingsModal({ isOpen, onClose }: SettingsModalProps) {
  const { t } = useTranslation();
  const [activeSection, setActiveSection] = useState<SettingsView>('appearance');

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
              className={`settings-sidebar-item ${activeSection === 'appearance' ? 'active' : ''}`}
              onClick={() => setActiveSection('appearance')}
            >
              <svg width="20" height="20" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                <circle cx="12" cy="12" r="5" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
                <path d="M12 1v2M12 21v2M4.22 4.22l1.42 1.42M18.36 18.36l1.42 1.42M1 12h2M21 12h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
              </svg>
              <span>{t('settings.appearance')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'preferences' ? 'active' : ''}`}
              onClick={() => setActiveSection('preferences')}
            >
              <Settings size={20} />
              <span>{t('settings.preferences')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'storage' ? 'active' : ''}`}
              onClick={() => setActiveSection('storage')}
            >
              <HardDrive size={20} />
              <span>{t('settings.storage')}</span>
            </button>

            <button
              className={`settings-sidebar-item ${activeSection === 'developer' ? 'active' : ''}`}
              onClick={() => setActiveSection('developer')}
            >
              <Terminal size={20} />
              <span>{t('settings.developer')}</span>
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
            {activeSection === 'appearance' && <AppearanceSection />}
            {activeSection === 'preferences' && <PreferencesSection />}
            {activeSection === 'storage' && <StorageSection />}
            {activeSection === 'developer' && <DeveloperSection />}
            {activeSection === 'about' && <AboutSection />}
          </div>
        </div>
      </div>
    </div>
  );
}
