import { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import { Lightbulb, Download, ShieldOff, Cpu, Sparkles, WifiOff, ShieldAlert } from 'lucide-react';
import {
  getShowMotd,
  setShowMotd,
  getShowUpdaterModal,
  setShowUpdaterModal,
  getShowWelcome,
  setShowWelcome,
  getSkipVerify,
  setSkipVerify,
  getForceOffline,
  setForceOffline,
  getAllowSystemDevices,
  setAllowSystemDevices,
  getArmbianBoardDetection,
  setArmbianBoardDetection,
} from '../../hooks/useSettings';
import { getSystemInfo, getArmbianRelease } from '../../hooks/useTauri';
import { useToasts } from '../../hooks/useToasts';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { EVENTS } from '../../config';

/** Preferences tab (notification/verification/connectivity/Armbian board-detection cards). Notifications use set-after-await; skip-verify & force-offline use optimistic update+rollback under a shared `isToggling` guard; the Armbian select uses optimistic update+rollback and is force-disabled (`'disabled'`) on non-Armbian/non-Linux hosts.
 * MOTD changes dispatch `MOTD_CHANGED`, all others `SETTINGS_CHANGED`; rendering gated until settings load to avoid toggle flicker on mount. */
export function PreferencesSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const settingsGroup = useSettingsGroup<{
    showMotd: boolean;
    showUpdaterModal: boolean;
    showWelcome: boolean;
    skipVerify: boolean;
    forceOffline: boolean;
    allowSystemDevices: boolean;
    armbianDetection: string;
    isArmbian: boolean;
  }>({
    showMotd: getShowMotd,
    showUpdaterModal: getShowUpdaterModal,
    showWelcome: getShowWelcome,
    skipVerify: getSkipVerify,
    forceOffline: getForceOffline,
    allowSystemDevices: getAllowSystemDevices,
    armbianDetection: getArmbianBoardDetection,
    isArmbian: async () => {
      const info = await getSystemInfo();
      if (info.platform !== 'linux') return false;
      const release = await getArmbianRelease();
      return release !== null;
    },
  });

  // Gates rendering until loaded to prevent toggle animation on mount
  const loaded = Object.keys(settingsGroup).length > 0;

  const [showMotd, setShowMotdState] = useState<boolean>(true);
  const [showUpdaterModal, setShowUpdaterModalState] = useState<boolean>(true);
  const [showWelcome, setShowWelcomeState] = useState<boolean>(true);
  const [skipVerify, setSkipVerifyState] = useState<boolean>(false);
  const [forceOffline, setForceOfflineState] = useState<boolean>(false);
  const [allowSystemDevices, setAllowSystemDevicesState] = useState<boolean>(false);
  const [armbianDetection, setArmbianDetection] = useState<string>('disabled');
  const [isToggling, setIsToggling] = useState<boolean>(false);
  const [initialized, setInitialized] = useState(false);

  // Initialize local state once settings load
  useEffect(() => {
    if (!loaded) return;
    if (settingsGroup.showMotd !== undefined) setShowMotdState(settingsGroup.showMotd);
    if (settingsGroup.showUpdaterModal !== undefined) setShowUpdaterModalState(settingsGroup.showUpdaterModal);
    if (settingsGroup.showWelcome !== undefined) setShowWelcomeState(settingsGroup.showWelcome);
    if (settingsGroup.skipVerify !== undefined) setSkipVerifyState(settingsGroup.skipVerify);
    if (settingsGroup.forceOffline !== undefined) setForceOfflineState(settingsGroup.forceOffline);
    if (settingsGroup.allowSystemDevices !== undefined) setAllowSystemDevicesState(settingsGroup.allowSystemDevices);
    if (settingsGroup.armbianDetection !== undefined) setArmbianDetection(settingsGroup.armbianDetection);
    setInitialized(true);
  }, [loaded, settingsGroup]);

  /** Toggles MOTD visibility (set-after-await), then dispatches MOTD_CHANGED. */
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

  /** Toggles the updater modal preference (set-after-await), then dispatches SETTINGS_CHANGED. */
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

  /** Toggles the welcome screen preference (set-after-await), then dispatches SETTINGS_CHANGED. */
  const handleToggleWelcome = async () => {
    try {
      const newValue = !showWelcome;
      await setShowWelcome(newValue);
      setShowWelcomeState(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.welcomeUpdated'));
    } catch (error) {
      console.error('Failed to set welcome screen preference:', error);
      showError(t('settings.toast.welcomeError'));
    }
  };

  /** Toggles skip-verify optimistically, rolling back on failure; guarded by isToggling. */
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

  /** Toggles force-offline optimistically, rolling back on failure; guarded by isToggling. */
  const handleToggleForceOffline = async () => {
    if (isToggling) return;

    const previousValue = forceOffline;
    const newValue = !forceOffline;
    setForceOfflineState(newValue);
    setIsToggling(true);

    try {
      await setForceOffline(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.forceOfflineUpdated'));
    } catch (error) {
      console.error('Failed to set force offline preference:', error);
      setForceOfflineState(previousValue);
      showError(t('settings.toast.forceOfflineError'));
    } finally {
      setIsToggling(false);
    }
  };

  /** Toggles allow-system-devices optimistically, rolling back on failure; guarded by isToggling. */
  const handleToggleAllowSystemDevices = async () => {
    if (isToggling) return;

    const previousValue = allowSystemDevices;
    const newValue = !allowSystemDevices;
    setAllowSystemDevicesState(newValue);
    setIsToggling(true);

    try {
      await setAllowSystemDevices(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.allowSystemDevicesUpdated'));
    } catch (error) {
      console.error('Failed to set allow system devices preference:', error);
      setAllowSystemDevicesState(previousValue);
      showError(t('settings.toast.allowSystemDevicesError'));
    } finally {
      setIsToggling(false);
    }
  };

  /** Updates Armbian board-detection mode (from select `e`) optimistically, rolling back on failure. */
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

  if (!initialized) return null;

  return (
    <div className="settings-section">
      {/* Notifications: MOTD, updater modal, welcome screen */}
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.notifications.title')}</h4>
        <div className="settings-group__card">
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <Lightbulb size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">
                  {t('settings.notifications.showMotd')}
                </div>
                <div className="settings-row__desc">
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

          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <Download size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">
                  {t('settings.notifications.showUpdaterModal')}
                </div>
                <div className="settings-row__desc">
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

          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <Sparkles size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">
                  {t('settings.notifications.showWelcome')}
                </div>
                <div className="settings-row__desc">
                  {t('settings.notifications.showWelcomeDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showWelcome}
                onChange={handleToggleWelcome}
                aria-label={t('settings.notifications.showWelcome')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* Verification: skip post-flash verification */}
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.verification')}</h4>
        <div className="settings-group__card">
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <ShieldOff size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.skipVerify')}</div>
                <div className="settings-row__desc">{t('settings.skipVerifyDescription')}</div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={skipVerify}
                onChange={handleToggleSkipVerify}
                disabled={isToggling}
                aria-label={t('settings.skipVerify')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* Connectivity: force offline mode */}
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.connectivity')}</h4>
        <div className="settings-group__card">
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <WifiOff size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.forceOffline')}</div>
                <div className="settings-row__desc">{t('settings.forceOfflineDescription')}</div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={forceOffline}
                onChange={handleToggleForceOffline}
                disabled={isToggling}
                aria-label={t('settings.forceOffline')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* Devices: unlock internal/system drives for flashing */}
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.devices')}</h4>
        <div className="settings-group__card">
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <ShieldAlert size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.allowSystemDevices')}</div>
                <div className="settings-row__desc">{t('settings.allowSystemDevicesDescription')}</div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={allowSystemDevices}
                onChange={handleToggleAllowSystemDevices}
                disabled={isToggling}
                aria-label={t('settings.allowSystemDevices')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* Armbian: board-detection mode (disabled on non-Armbian/non-Linux hosts) */}
      <div className="settings-group">
        <h4 className="settings-group__title">{t('settings.armbian.title')}</h4>
        <div className="settings-group__card">
          <div className="settings-row">
            <div className="settings-row__main">
              <div className="settings-row__icon">
                <Cpu size={18} />
              </div>
              <div className="settings-row__text">
                <div className="settings-row__label">{t('settings.armbian.label')}</div>
                <div className="settings-row__desc">
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
