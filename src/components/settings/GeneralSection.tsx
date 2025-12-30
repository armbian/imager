import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Lightbulb, Download } from 'lucide-react';
import { getShowMotd, setShowMotd, getShowUpdaterModal, setShowUpdaterModal } from '../../hooks/useSettings';

/**
 * General settings section for sidebar layout
 *
 * Contains general app preferences like MOTD visibility.
 */
export function GeneralSection() {
  const { t } = useTranslation();
  const [showMotd, setShowMotdState] = useState<boolean>(true);
  const [showUpdaterModal, setShowUpdaterModalState] = useState<boolean>(true);

  // Load MOTD preference on mount
  useEffect(() => {
    const loadMotdPreference = async () => {
      try {
        const value = await getShowMotd();
        setShowMotdState(value);
      } catch (error) {
        console.error('Failed to load MOTD preference:', error);
      }
    };
    loadMotdPreference();
  }, []);

  // Load updater modal preference on mount
  useEffect(() => {
    const loadUpdaterModalPreference = async () => {
      try {
        const value = await getShowUpdaterModal();
        setShowUpdaterModalState(value);
      } catch (error) {
        console.error('Failed to load updater modal preference:', error);
      }
    };
    loadUpdaterModalPreference();
  }, []);

  const handleToggleMotd = async () => {
    try {
      const newValue = !showMotd;
      await setShowMotd(newValue);
      setShowMotdState(newValue);

      // Notify MOTD component specifically
      window.dispatchEvent(new Event('armbian-motd-changed'));
    } catch (error) {
      console.error('Failed to set MOTD preference:', error);
    }
  };

  const handleToggleUpdaterModal = async () => {
    try {
      const newValue = !showUpdaterModal;
      await setShowUpdaterModal(newValue);
      setShowUpdaterModalState(newValue);

      // Notify other components that settings changed
      window.dispatchEvent(new Event('armbian-settings-changed'));
    } catch (error) {
      console.error('Failed to set updater modal preference:', error);
    }
  };

  return (
    <div className="settings-section">
      <h3 className="settings-section-title">{t('settings.generalTitle')}</h3>

      <div className="settings-list">
        <div className="settings-item">
          <div className="settings-item-left">
            <div className="settings-item-icon">
              <Lightbulb />
            </div>
            <div className="settings-item-content">
              <div className="settings-item-label">{t('settings.showMotd')}</div>
              <div className="settings-item-description">{t('settings.showMotdDescription')}</div>
            </div>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={showMotd}
              onChange={handleToggleMotd}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
        <div className="settings-item">
          <div className="settings-item-left">
            <div className="settings-item-icon">
              <Download />
            </div>
            <div className="settings-item-content">
              <div className="settings-item-label">{t('settings.showUpdaterModal')}</div>
              <div className="settings-item-description">{t('settings.showUpdaterModalDescription')}</div>
            </div>
          </div>
          <label className="toggle-switch">
            <input
              type="checkbox"
              checked={showUpdaterModal}
              onChange={handleToggleUpdaterModal}
            />
            <span className="toggle-slider"></span>
          </label>
        </div>
      </div>
    </div>
  );
}
