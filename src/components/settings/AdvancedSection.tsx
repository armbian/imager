import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Code, FileText } from 'lucide-react';
import { getDeveloperMode, setDeveloperMode } from '../../hooks/useSettings';
import { LogsModal } from './LogsModal';
import { EVENTS } from '../../config';

/**
 * Advanced settings section for power users
 *
 * Contains developer mode toggle and view logs button.
 */
export function AdvancedSection() {
  const { t } = useTranslation();
  const [developerMode, setDeveloperModeState] = useState<boolean>(false);
  const [logsModalOpen, setLogsModalOpen] = useState<boolean>(false);
  const [isToggling, setIsToggling] = useState<boolean>(false);

  // Load developer mode preference on mount
  useEffect(() => {
    const loadDeveloperModePreference = async () => {
      try {
        const value = await getDeveloperMode();
        setDeveloperModeState(value);
      } catch (error) {
        console.error('Failed to load developer mode preference:', error);
      }
    };
    loadDeveloperModePreference();
  }, []);

  const handleToggleDeveloperMode = async () => {
    // Prevent concurrent toggles
    if (isToggling) {
      return;
    }

    const previousValue = developerMode;
    const newValue = !developerMode;

    // Optimistic update
    setDeveloperModeState(newValue);
    setIsToggling(true);

    try {
      await setDeveloperMode(newValue);

      // Notify other components that settings changed
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
    } catch (error) {
      // Rollback on error
      console.error('Failed to set developer mode preference:', error);
      setDeveloperModeState(previousValue);
    } finally {
      setIsToggling(false);
    }
  };

  return (
    <div className="settings-section">
      <h3 className="settings-section-title">{t('settings.advancedCategory')}</h3>

      <div className="settings-list">
        {/* Developer Mode Toggle */}
        <div className="settings-item">
          <div className="settings-item-left">
            <div className="settings-item-icon">
              <Code />
            </div>
            <div className="settings-item-content">
              <div className="settings-item-label">{t('settings.developerMode')}</div>
              <div className="settings-item-description">{t('settings.developerModeDescription')}</div>
            </div>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={developerMode}
              onChange={handleToggleDeveloperMode}
              disabled={isToggling}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>

        {/* View Logs Button */}
        <div className="settings-item settings-item-clickable" onClick={() => setLogsModalOpen(true)}>
          <div className="settings-item-left">
            <div className="settings-item-icon">
              <FileText />
            </div>
            <div className="settings-item-content">
              <div className="settings-item-label">{t('settings.viewLogs')}</div>
              <div className="settings-item-description">{t('settings.viewLogsDescription')}</div>
            </div>
          </div>
          <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <polyline points="9 18 15 12 9 6"></polyline>
          </svg>
        </div>
      </div>

      {/* Logs Modal */}
      <LogsModal isOpen={logsModalOpen} onClose={() => setLogsModalOpen(false)} />
    </div>
  );
}
