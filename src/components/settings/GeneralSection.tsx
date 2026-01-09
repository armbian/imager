import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Lightbulb, Download, HardDrive, Database, Trash2 } from 'lucide-react';
import {
  getShowMotd,
  setShowMotd,
  getShowUpdaterModal,
  setShowUpdaterModal,
  getCacheEnabled,
  setCacheEnabled,
  getCacheMaxSize,
  setCacheMaxSize,
} from '../../hooks/useSettings';
import { getCacheSize, clearCache } from '../../hooks/useTauri';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
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
 * Contains notification preferences and cache management.
 */
export function GeneralSection() {
  const { t } = useTranslation();

  // Notification states
  const [showMotd, setShowMotdState] = useState<boolean>(true);
  const [showUpdaterModal, setShowUpdaterModalState] = useState<boolean>(true);

  // Cache states
  const [cacheEnabled, setCacheEnabledState] = useState<boolean>(true);
  const [cacheMaxSize, setCacheMaxSizeState] = useState<number>(CACHE.DEFAULT_SIZE);
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

  // Load cache preferences on mount
  useEffect(() => {
    const loadCachePreferences = async () => {
      try {
        const [enabled, maxSize] = await Promise.all([
          getCacheEnabled(),
          getCacheMaxSize(),
        ]);
        setCacheEnabledState(enabled);
        setCacheMaxSizeState(maxSize);
      } catch (error) {
        console.error('Failed to load cache preferences:', error);
      }
    };
    loadCachePreferences();
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
    } catch (error) {
      console.error('Failed to set MOTD preference:', error);
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
    } catch (error) {
      console.error('Failed to set updater modal preference:', error);
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
    } catch (error) {
      console.error('Failed to set cache enabled preference:', error);
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
    } catch (error) {
      console.error('Failed to set cache max size:', error);
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
    } catch {
      // Error already logged by clearCache
    } finally {
      setIsClearing(false);
    }
  };

  return (
    <div className="settings-section">
      {/* NOTIFICATIONS Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">
          {t('settings.notificationsCategory')}
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
                  {t('settings.showMotd')}
                </div>
                <div className="settings-item-description">
                  {t('settings.showMotdDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showMotd}
                onChange={handleToggleMotd}
                aria-label={t('settings.showMotd')}
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
                  {t('settings.showUpdaterModal')}
                </div>
                <div className="settings-item-description">
                  {t('settings.showUpdaterModalDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={showUpdaterModal}
                onChange={handleToggleUpdaterModal}
                aria-label={t('settings.showUpdaterModal')}
              />
              <span className="toggle-slider"></span>
            </label>
          </div>
        </div>
      </div>

      {/* CACHE Section */}
      <div className="settings-category">
        <h4 className="settings-category-title">
          {t('settings.cacheCategory')}
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
                  {t('settings.enableCache')}
                </div>
                <div className="settings-item-description">
                  {t('settings.enableCacheDescription')}
                </div>
              </div>
            </div>
            <label className="toggle-switch">
              <input
                type="checkbox"
                checked={cacheEnabled}
                onChange={handleToggleCacheEnabled}
                aria-label={t('settings.enableCache')}
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
                  {t('settings.maxCacheSize')}
                </div>
                <div className="settings-item-description">
                  {t('settings.maxCacheSizeDescription')}
                </div>
              </div>
            </div>
            <select
              className="settings-select"
              value={cacheMaxSize}
              onChange={handleCacheMaxSizeChange}
              disabled={!cacheEnabled}
              aria-label={t('settings.maxCacheSize')}
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
                  {t('settings.cacheSize')}
                </div>
                <div className="settings-item-description">
                  {isLoadingCacheSize
                    ? t('modal.loading')
                    : currentCacheSize === 0
                      ? t('settings.noCachedImages')
                      : formatBytes(currentCacheSize)}
                </div>
              </div>
            </div>
            <button
              className="btn btn-secondary btn-sm"
              onClick={handleClearCacheClick}
              disabled={isClearing || currentCacheSize === 0}
              aria-label={t('settings.clearCache')}
            >
              {isClearing ? t('modal.loading') : t('settings.clearCache')}
            </button>
          </div>
        </div>
      </div>

      {/* Clear cache confirmation dialog */}
      <ConfirmationDialog
        isOpen={showClearConfirm}
        title={t('settings.clearCache')}
        message={t('settings.clearCacheConfirm')}
        confirmText={t('common.confirm')}
        isDanger={true}
        onCancel={() => setShowClearConfirm(false)}
        onConfirm={handleClearCacheConfirm}
      />
    </div>
  );
}
