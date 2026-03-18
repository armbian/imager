import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { HardDrive, Database, Trash2, FolderOpen, ChevronRight } from 'lucide-react';
import {
  getCacheEnabled,
  setCacheEnabled,
  getCacheMaxSize,
  setCacheMaxSize,
} from '../../hooks/useSettings';
import { getCacheSize, clearCache } from '../../hooks/useTauri';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { CacheManagerModal } from './CacheManagerModal';
import { useToasts } from '../../hooks/useToasts';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { CACHE, EVENTS } from '../../config';
import { formatBytes } from '../../utils';

/**
 * Storage settings section for cache management
 *
 * Contains cache enable toggle, max size dropdown, clear cache, and manage cached images.
 */
export function StorageSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  // Load cache-related settings on mount
  const settingsGroup = useSettingsGroup<{
    cacheEnabled: boolean;
    cacheMaxSize: number;
  }>({
    cacheEnabled: getCacheEnabled,
    cacheMaxSize: getCacheMaxSize,
  });

  // Local state for mutable values
  const [cacheEnabled, setCacheEnabledState] = useState<boolean>(true);
  const [cacheMaxSize, setCacheMaxSizeState] = useState<number>(CACHE.DEFAULT_SIZE);
  const [initialized, setInitialized] = useState(false);

  // Sync with loaded values once
  useEffect(() => {
    if (Object.keys(settingsGroup).length === 0) return;
    if (settingsGroup.cacheEnabled !== undefined) setCacheEnabledState(settingsGroup.cacheEnabled);
    if (settingsGroup.cacheMaxSize !== undefined) setCacheMaxSizeState(settingsGroup.cacheMaxSize);
    setInitialized(true);
  }, [settingsGroup]);

  // Local state for non-persistent values
  const [currentCacheSize, setCurrentCacheSize] = useState<number>(0);
  const [isClearing, setIsClearing] = useState<boolean>(false);
  const [isLoadingCacheSize, setIsLoadingCacheSize] = useState<boolean>(true);
  const [showClearConfirm, setShowClearConfirm] = useState<boolean>(false);
  const [cacheManagerOpen, setCacheManagerOpen] = useState<boolean>(false);

  /** Load current cache size from backend */
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

  // Load cache size on mount
  useEffect(() => {
    loadCacheSize();
  }, [loadCacheSize]);

  /** Toggle cache enabled/disabled */
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

  /** Handle cache max size change from dropdown */
  const handleCacheMaxSizeChange = async (
    e: React.ChangeEvent<HTMLSelectElement>
  ) => {
    try {
      const newSize = parseInt(e.target.value, 10);
      await setCacheMaxSize(newSize);
      setCacheMaxSizeState(newSize);
      loadCacheSize();
      showSuccess(t('settings.toast.cacheSizeUpdated'));
    } catch (error) {
      console.error('Failed to set cache max size:', error);
      showError(t('settings.toast.cacheSizeError'));
    }
  };

  /** Show confirmation dialog before clearing cache */
  const handleClearCacheClick = () => {
    setShowClearConfirm(true);
  };

  /** Clear all cached images after user confirmation */
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

  if (!initialized) return null;

  return (
    <>
      <div className="settings-section">
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

            {/* Manage cached images */}
            <div
              className="settings-item settings-item-clickable"
              onClick={() => setCacheManagerOpen(true)}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  setCacheManagerOpen(true);
                }
              }}
            >
              <div className="settings-item-left">
                <div className="settings-item-icon">
                  <FolderOpen />
                </div>
                <div className="settings-item-content">
                  <div className="settings-item-label">
                    {t('settings.cache.manage')}
                  </div>
                  <div className="settings-item-description">
                    {t('settings.cache.manageDescription')}
                  </div>
                </div>
              </div>
              <div className="settings-item-arrow">
                <ChevronRight size={20} />
              </div>
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

        {/* Cache Manager modal */}
        <CacheManagerModal
          isOpen={cacheManagerOpen}
          onClose={() => {
            setCacheManagerOpen(false);
            loadCacheSize();
          }}
        />
      </div>
    </>
  );
}
