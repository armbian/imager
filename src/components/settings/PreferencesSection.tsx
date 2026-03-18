import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Lightbulb, Download, ShieldOff, Cpu } from 'lucide-react';
import {
  getShowMotd,
  setShowMotd,
  getShowUpdaterModal,
  setShowUpdaterModal,
  getSkipVerify,
  setSkipVerify,
  getArmbianBoardDetection,
  setArmbianBoardDetection,
} from '../../hooks/useSettings';
import { getSystemInfo, getArmbianRelease } from '../../hooks/useTauri';
import { useToasts } from '../../hooks/useToasts';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { EVENTS } from '../../config';

/**
 * Preferences settings section
 *
 * Contains notification preferences, verification settings, and Armbian board detection.
 */
export function PreferencesSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  // Load all persistent settings and Armbian detection on mount
  const settingsGroup = useSettingsGroup<{
    showMotd: boolean;
    showUpdaterModal: boolean;
    skipVerify: boolean;
    armbianDetection: string;
    isArmbian: boolean;
  }>({
    showMotd: getShowMotd,
    showUpdaterModal: getShowUpdaterModal,
    skipVerify: getSkipVerify,
    armbianDetection: getArmbianBoardDetection,
    isArmbian: async () => {
      const info = await getSystemInfo();
      if (info.platform !== 'linux') return false;
      const release = await getArmbianRelease();
      return release !== null;
    },
  });

  // Track whether initial load is complete to avoid switch animation on mount
  const loaded = Object.keys(settingsGroup).length > 0;

  // Local state for mutable values — initialized from loaded settings
  const [showMotd, setShowMotdState] = useState<boolean>(true);
  const [showUpdaterModal, setShowUpdaterModalState] = useState<boolean>(true);
  const [skipVerify, setSkipVerifyState] = useState<boolean>(false);
  const [armbianDetection, setArmbianDetection] = useState<string>('disabled');
  const [isToggling, setIsToggling] = useState<boolean>(false);
  const [initialized, setInitialized] = useState(false);

  // Sync with loaded values once, then mark initialized
  useEffect(() => {
    if (!loaded) return;
    if (settingsGroup.showMotd !== undefined) setShowMotdState(settingsGroup.showMotd);
    if (settingsGroup.showUpdaterModal !== undefined) setShowUpdaterModalState(settingsGroup.showUpdaterModal);
    if (settingsGroup.skipVerify !== undefined) setSkipVerifyState(settingsGroup.skipVerify);
    if (settingsGroup.armbianDetection !== undefined) setArmbianDetection(settingsGroup.armbianDetection);
    setInitialized(true);
  }, [loaded, settingsGroup]);

  /** Toggle MOTD visibility */
  const handleToggleMotd = async () => {
    try {
      const newValue = !showMotd;
      await setShowMotd(newValue);
      setShowMotdState(newValue);
      window.dispatchEvent(new Event(EVENTS.MOTD_CHANGED));
      showSuccess(t('settings.toast.motdUpdated'));
    } catch (error) {
      console.error('Failed to set MOTD preference:', error);
      showError(t('settings.toast.motdError'));
    }
  };

  /** Toggle updater modal visibility */
  const handleToggleUpdaterModal = async () => {
    try {
      const newValue = !showUpdaterModal;
      await setShowUpdaterModal(newValue);
      setShowUpdaterModalState(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.updaterUpdated'));
    } catch (error) {
      console.error('Failed to set updater modal preference:', error);
      showError(t('settings.toast.updaterError'));
    }
  };

  /** Toggle skip verification */
  const handleToggleSkipVerify = async () => {
    if (isToggling) return;

    const previousValue = skipVerify;
    const newValue = !skipVerify;
    setSkipVerifyState(newValue);
    setIsToggling(true);

    try {
      await setSkipVerify(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.skipVerifyUpdated'));
    } catch (error) {
      console.error('Failed to set skip verify preference:', error);
      setSkipVerifyState(previousValue);
      showError(t('settings.toast.skipVerifyError'));
    } finally {
      setIsToggling(false);
    }
  };

  /** Handle Armbian board detection mode change */
  const handleArmbianDetectionChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const previousMode = armbianDetection;
    const newMode = e.target.value;

    try {
      await setArmbianBoardDetection(newMode);
      setArmbianDetection(newMode);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.detectionUpdated'));
    } catch (error) {
      console.error('Failed to set Armbian detection preference:', error);
      setArmbianDetection(previousMode);
      showError(t('settings.toast.detectionError'));
    }
  };

  // Don't render until settings are loaded to prevent switch animation
  if (!initialized) return null;

  return (
    <div className="settings-section">
      {/* NOTIFICATIONS Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">
          {t('settings.notifications.title')}
        </h4>
        <div className="settings-list">
          {/* Show tips toggle */}
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <Lightbulb />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">
                  {t('settings.notifications.showMotd')}
                </div>
                <div className="settings-item-description">
                  {t('settings.notifications.showMotdDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showMotd}
                onChange={handleToggleMotd}
                aria-label={t('settings.notifications.showMotd')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>

          {/* Show update notifications toggle */}
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <Download />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">
                  {t('settings.notifications.showUpdaterModal')}
                </div>
                <div className="settings-item-description">
                  {t('settings.notifications.showUpdaterModalDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showUpdaterModal}
                onChange={handleToggleUpdaterModal}
                aria-label={t('settings.notifications.showUpdaterModal')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* VERIFICATION Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">
          {t('settings.verification')}
        </h4>
        <div className="settings-list">
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <ShieldOff />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">{t('settings.skipVerify')}</div>
                <div className="settings-item-description">{t('settings.skipVerifyDescription')}</div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={skipVerify}
                onChange={handleToggleSkipVerify}
                disabled={isToggling}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* ARMBIAN DETECTION Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">{t('settings.armbian.title')}</h4>
        <div className="settings-list">
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <Cpu />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">{t('settings.armbian.label')}</div>
                <div className="settings-item-description">
                  {t('settings.armbian.description')}
                </div>
              </div>
            </div>
            <select
              className="settings-select"
              value={settingsGroup.isArmbian ? armbianDetection : 'disabled'}
              onChange={handleArmbianDetectionChange}
              disabled={!settingsGroup.isArmbian}
              aria-label={t('settings.armbian.label')}
            >
              <option value="disabled">{t('settings.armbian.mode_disabled')}</option>
              <option value="modal">{t('settings.armbian.mode_modal')}</option>
              <option value="auto">{t('settings.armbian.mode_auto')}</option>
            </select>
          </div>
        </div>
      </div>
    </div>
  );
}
