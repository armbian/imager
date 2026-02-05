import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Lightbulb, Download, HardDrive, Database, Cpu, Trash2 } from 'lucide-react';
import {
  getShowMotd,
  setShowMotd,
  getShowUpdaterModal,
  setShowUpdaterModal,
  getCacheEnabled,
  setCacheEnabled,
  getCacheMaxSize,
  setCacheMaxSize,
  getArmbianBoardDetection,
  setArmbianBoardDetection,
} from '../../hooks/useSettings';
import { getCacheSize, clearCache } from '../../hooks/useTauri';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { useToasts } from '../../hooks/useToasts';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { CACHE, EVENTS } from '../../config';

/**
 * Format bytes to human-readable string
 *
 * @param bytes - Size in bytes
 * @returns Formatted string (e.g., "2.3 GB")
 */
function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

/**
 * General settings section for sidebar layout
 *
 * Contains notification preferences, cache management, and Armbian detection settings.
 */
export function GeneralSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  // Load all persistent settings on mount using useSettingsGroup hook
  const settingsGroup = useSettingsGroup<{
    showMotd: boolean;
    showUpdaterModal: boolean;
    cacheEnabled: boolean;
    cacheMaxSize: number;
    armbianDetection: string;
  }>({
    showMotd: getShowMotd,
    showUpdaterModal: getShowUpdaterModal,
    cacheEnabled: getCacheEnabled,
    cacheMaxSize: getCacheMaxSize,
    armbianDetection: getArmbianBoardDetection,
  });

  // Local state for mutable values (to allow UI updates before persistence)
  const [showMotd, setShowMotdState] = useState<boolean>(settingsGroup.showMotd ?? true);
  const [showUpdaterModal, setShowUpdaterModalState] = useState<boolean>(settingsGroup.showUpdaterModal ?? true);
  const [cacheEnabled, setCacheEnabledState] = useState<boolean>(settingsGroup.cacheEnabled ?? true);
  const [cacheMaxSize, setCacheMaxSizeState] = useState<number>(settingsGroup.cacheMaxSize ?? CACHE.DEFAULT_SIZE);
  const [armbianDetection, setArmbianDetection] = useState<string>(settingsGroup.armbianDetection ?? 'disabled');

  // Sync with loaded values
  useEffect(() => {
    if (settingsGroup.showMotd !== undefined) setShowMotdState(settingsGroup.showMotd);
    if (settingsGroup.showUpdaterModal !== undefined) setShowUpdaterModalState(settingsGroup.showUpdaterModal);
    if (settingsGroup.cacheEnabled !== undefined) setCacheEnabledState(settingsGroup.cacheEnabled);
    if (settingsGroup.cacheMaxSize !== undefined) setCacheMaxSizeState(settingsGroup.cacheMaxSize);
    if (settingsGroup.armbianDetection !== undefined) setArmbianDetection(settingsGroup.armbianDetection);
  }, [settingsGroup]);

  // Local state for non-persistent values
  const [currentCacheSize, setCurrentCacheSize] = useState<number>(0);
  const [isClearing, setIsClearing] = useState<boolean>(false);
  const [isLoadingCacheSize, setIsLoadingCacheSize] = useState<boolean>(true);
  const [showClearConfirm, setShowClearConfirm] = useState<boolean>(false);

  /**
   * Load current cache size from backend
   */
  const loadCacheSize = useCallback(async () => {
    try {
      setIsLoadingCacheSize(true);
      const size = await getCacheSize();
      setCurrentCacheSize(size);
    } catch (error) {
      console.error('Failed to load cache size:', error);
    } finally {
      setIsLoadingCacheSize(false);
    }
  }, []);

  // Load cache size on mount (separate from useSettingsGroup since it's not a setting)
  useEffect(() => {
    loadCacheSize();
  }, [loadCacheSize]);

  /**
   * Toggle MOTD visibility
   */
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

  /**
   * Toggle updater modal visibility
   */
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

  /**
   * Toggle cache enabled/disabled
   */
  const handleToggleCacheEnabled = async () => {
    try {
      const newValue = !cacheEnabled;
      await setCacheEnabled(newValue);
      setCacheEnabledState(newValue);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.cacheToggleUpdated'));
    } catch (error) {
      console.error('Failed to set cache enabled preference:', error);
      showError(t('settings.toast.cacheToggleError'));
    }
  };

  /**
   * Handle cache max size change from dropdown
   */
  const handleCacheMaxSizeChange = async (
    e: React.ChangeEvent<HTMLSelectElement>
  ) => {
    try {
      const newSize = parseInt(e.target.value, 10);
      await setCacheMaxSize(newSize);
      setCacheMaxSizeState(newSize);
      // Reload cache size in case eviction happened
      loadCacheSize();
      showSuccess(t('settings.toast.cacheSizeUpdated'));
    } catch (error) {
      console.error('Failed to set cache max size:', error);
      showError(t('settings.toast.cacheSizeError'));
    }
  };

  /**
   * Show confirmation dialog before clearing cache
   */
  const handleClearCacheClick = () => {
    setShowClearConfirm(true);
  };

  /**
   * Clear all cached images after user confirmation
   */
  const handleClearCacheConfirm = async () => {
    setShowClearConfirm(false);
    try {
      setIsClearing(true);
      await clearCache();
      setCurrentCacheSize(0);
      showSuccess(t('settings.toast.cacheClearSuccess'));
    } catch {
      showError(t('settings.toast.cacheClearError'));
    } finally {
      setIsClearing(false);
    }
  };

  /**
   * Handle Armbian board detection mode change
   * Reverts to previous value on save failure
   */
  const handleArmbianDetectionChange = async (e: React.ChangeEvent<HTMLSelectElement>) => {
    const previousMode = armbianDetection; // Store previous value for revert
    const newMode = e.target.value;

    try {
      await setArmbianBoardDetection(newMode);
      setArmbianDetection(newMode);
      window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
      showSuccess(t('settings.toast.detectionUpdated'));
    } catch (error) {
      console.error('Failed to set Armbian detection preference:', error);
      // Revert to previous value on error
      setArmbianDetection(previousMode);
      showError(t('settings.toast.detectionError'));
    }
  };

  return (
    <>
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

      {/* CACHE Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">
          {t('settings.cache.title')}
        </h4>
        <div className="settings-list">
          {/* Enable image cache toggle */}
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <HardDrive />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">
                  {t('settings.cache.enable')}
                </div>
                <div className="settings-item-description">
                  {t('settings.cache.enableDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={cacheEnabled}
                onChange={handleToggleCacheEnabled}
                aria-label={t('settings.cache.enable')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>

          {/* Maximum cache size dropdown */}
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <Database />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">
                  {t('settings.cache.maxSize')}
                </div>
                <div className="settings-item-description">
                  {t('settings.cache.maxSizeDescription')}
                </div>
              </div>
            </div>
            <select
              className="settings-select"
              value={cacheMaxSize}
              onChange={handleCacheMaxSizeChange}
              disabled={!cacheEnabled}
              aria-label={t('settings.cache.maxSize')}
            >
              {CACHE.SIZE_OPTIONS.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </div>

          {/* Cache size display with clear button */}
          <div className="settings-item">
            <div className="settings-item-left">
              <div className="settings-item-icon">
                <Trash2 />
              </div>
              <div className="settings-item-content">
                <div className="settings-item-label">
                  {t('settings.cache.size')}
                </div>
                <div className="settings-item-description">
                  {isLoadingCacheSize
                    ? t('modal.loading')
                    : currentCacheSize === 0
                      ? t('settings.cache.noCachedImages')
                      : formatBytes(currentCacheSize)}
                </div>
              </div>
            </div>
            <button
              className="btn btn-secondary btn-sm"
              onClick={handleClearCacheClick}
              disabled={isClearing || currentCacheSize === 0}
              aria-label={t('settings.cache.clear')}
            >
              {isClearing ? t('modal.loading') : t('settings.cache.clear')}
            </button>
          </div>
        </div>
      </div>

      {/* ARMBIAN DETECTION Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">{t('settings.armbian.title')}</h4>
        <div className="settings-list">
          {/* Armbian board detection mode */}
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
              value={armbianDetection}
              onChange={handleArmbianDetectionChange}
              aria-label={t('settings.armbian.label')}
            >
              <option value="disabled">{t('settings.armbian.mode_disabled')}</option>
              <option value="modal">{t('settings.armbian.mode_modal')}</option>
              <option value="auto">{t('settings.armbian.mode_auto')}</option>
            </select>
          </div>
        </div>
      </div>

      {/* Clear cache confirmation dialog */}
      <ConfirmationDialog
        isOpen={showClearConfirm}
        title={t('settings.cache.clear')}
        message={t('settings.cache.clearConfirm')}
        confirmText={t('common.confirm')}
        isDanger={true}
        onCancel={() => setShowClearConfirm(false)}
        onConfirm={handleClearCacheConfirm}
      />
      </div>
    </>
  );
}
