import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, Code, FileText } from 'lucide-react';
import { getDeveloperMode, setDeveloperMode } from '../../hooks/useSettings';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { LogsModal } from './LogsModal';
import { EVENTS } from '../../config';

/** Developer settings: dev-mode toggle and session log viewer entry, using the shared
 * settings-group / settings-row glass vocabulary. */
export function DeveloperSection() {
  const { t } = useTranslation();
  const [developerMode, setDeveloperModeState] = useState<boolean>(false);
  const [logsModalOpen, setLogsModalOpen] = useState<boolean>(false);
  const [isToggling, setIsToggling] = useState<boolean>(false);
  const [initialized, setInitialized] = useState(false);

  const settingsGroup = useSettingsGroup<{
    developerMode: boolean;
  }>({
    developerMode: getDeveloperMode,
  });

  // Sync local state once the persisted developer-mode value is read.
  useEffect(() => {
    if (settingsGroup.developerMode !== undefined) {
      setDeveloperModeState(settingsGroup.developerMode);
      setInitialized(true);
    }
  }, [settingsGroup.developerMode]);

  /** Toggle developer mode with optimistic update and rollback on error */
  const handleToggleDeveloperMode = async () => {
    if (isToggling) return;

    const previousValue = developerMode;
    const newValue = !developerMode;

    setDeveloperModeState(newValue);
    setIsToggling(true);

    try {
      await setDeveloperMode(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
    } catch (error) {
      console.error('Failed to set developer mode preference:', error);
      setDeveloperModeState(previousValue);
    } finally {
      setIsToggling(false);
    }
  };

  // Gate rendering until the persisted setting has been loaded.
  if (!initialized) return null;

  return (
    <div className="settings-section">
      <div className="settings-group">
        <h3 className="settings-group__title">{t('settings.developer')}</h3>

        <div className="settings-group__card">
          {/* Developer-mode toggle */}
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <Code />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.developerMode')}</div>
                <div className="settings-row__desc">{t('settings.developerModeDescription')}</div>
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

          {/* Log viewer entry point */}
          <div
            className="settings-row settings-row--clickable"
            role="button"
            tabIndex={0}
            onClick={() => setLogsModalOpen(true)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' || e.key === ' ') {
                e.preventDefault();
                setLogsModalOpen(true);
              }
            }}
          >
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <FileText />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.viewLogs')}</div>
                <div className="settings-row__desc">{t('settings.viewLogsDescription')}</div>
              </div>
            </div>
            <ChevronRight className="settings-row__arrow" size={20} />
          </div>
        </div>
      </div>

      <LogsModal isOpen={logsModalOpen} onClose={() => setLogsModalOpen(false)} />
    </div>
  );
}
