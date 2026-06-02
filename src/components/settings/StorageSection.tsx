import { useState, useEffect, useCallback, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { HardDrive, Database, Trash2, FolderOpen, ChevronRight } from 'lucide-react';
import {
  getCacheEnabled,
  setCacheEnabled,
  getCacheMaxSize,
  setCacheMaxSize,
} from '../../hooks/useSettings';
import { getCacheBreakdown, clearCache } from '../../hooks/useTauri';
import type { CacheBreakdown } from '../../types';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { CacheManagerModal } from './CacheManagerModal';
import { useToasts } from '../../hooks/useToasts';
import { useSettingsGroup } from '../../hooks/useSettingsGroup';
import { CACHE, EVENTS } from '../../config';
import { formatBytes } from '../../utils';

/** Storage section: cache usage panel (hairline meter, coherent with `.flash-track`) plus a card of controls — enable toggle, max-size select, clear, and a row opening CacheManagerModal.
 * Preserves backend couplings: cache size reload on mount, after size change, and after the manager closes. */
export function StorageSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();

  const settingsGroup = useSettingsGroup<{
    cacheEnabled: boolean;
    cacheMaxSize: number;
  }>({
    cacheEnabled: getCacheEnabled,
    cacheMaxSize: getCacheMaxSize,
  });

  const [cacheEnabled, setCacheEnabledState] = useState<boolean>(true);
  const [cacheMaxSize, setCacheMaxSizeState] = useState<number>(CACHE.DEFAULT_SIZE);
  const [initialized, setInitialized] = useState(false);

  // Initialize local state once settings load
  useEffect(() => {
    if (Object.keys(settingsGroup).length === 0) return;
    if (settingsGroup.cacheEnabled !== undefined) setCacheEnabledState(settingsGroup.cacheEnabled);
    if (settingsGroup.cacheMaxSize !== undefined) setCacheMaxSizeState(settingsGroup.cacheMaxSize);
    setInitialized(true);
  }, [settingsGroup]);

  const [breakdown, setBreakdown] = useState<CacheBreakdown>({ images: 0, assets: 0, total: 0 });
  const [isClearing, setIsClearing] = useState<boolean>(false);
  const [isLoadingCacheSize, setIsLoadingCacheSize] = useState<boolean>(true);
  const [showClearConfirm, setShowClearConfirm] = useState<boolean>(false);
  const [cacheManagerOpen, setCacheManagerOpen] = useState<boolean>(false);

  // Total cache size in bytes, derived from the per-category breakdown
  const currentCacheSize = breakdown.total;

  /** Load the per-category cache breakdown (images vs assets) from backend */
  const loadCacheSize = useCallback(async () => {
    try {
      setIsLoadingCacheSize(true);
      const result = await getCacheBreakdown();
      setBreakdown(result);
    } catch (error) {
      console.error('Failed to load cache breakdown:', error);
    } finally {
      setIsLoadingCacheSize(false);
    }
  }, []);

  useEffect(() => {
    loadCacheSize();
  }, [loadCacheSize]);

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
      setBreakdown({ images: 0, assets: 0, total: 0 });
      showSuccess(t('settings.toast.cacheClearSuccess'));
    } catch {
      showError(t('settings.toast.cacheClearError'));
    } finally {
      setIsClearing(false);
    }
  };

  /** Open the full cache manager modal */
  const openCacheManager = () => setCacheManagerOpen(true);

  /** Human-readable label of the currently selected max-size limit */
  const limitLabel = useMemo(() => {
    const match = CACHE.SIZE_OPTIONS.find((option) => option.value === cacheMaxSize);
    return match ? match.label : formatBytes(cacheMaxSize);
  }, [cacheMaxSize]);

  /** Per-category fill % vs the limit; segments stack images-then-assets. Each non-empty category is floored to a
   * visible width so a tiny assets cache stays distinguishable, then combined width is clamped to never overflow the track. */
  const { imagesPercent, assetsPercent } = useMemo(() => {
    if (cacheMaxSize <= 0) return { imagesPercent: 0, assetsPercent: 0 };
    const MIN_VISIBLE = 4; // percent floor for a non-empty segment
    const rawImages = (breakdown.images / cacheMaxSize) * 100;
    const rawAssets = (breakdown.assets / cacheMaxSize) * 100;
    let images = breakdown.images > 0 ? Math.max(rawImages, MIN_VISIBLE) : 0;
    let assets = breakdown.assets > 0 ? Math.max(rawAssets, MIN_VISIBLE) : 0;
    // Keep the assets segment visible, then let images take the remaining room
    assets = Math.min(assets, 100);
    images = Math.min(images, 100 - assets);
    return { imagesPercent: images, assetsPercent: assets };
  }, [breakdown.images, breakdown.assets, cacheMaxSize]);

  if (!initialized) return null;

  return (
    <>
      <div className="settings-section">
        {/* Cache usage: titled glass panel with a hairline cache meter */}
        <div className="settings-group">
          <h4 className="settings-group__title">{t('settings.cache.usageTitle')}</h4>
          <div className="storage-usage">
          <div className="storage-usage__head">
            <span className="storage-usage__value">
              {isLoadingCacheSize
                ? t('modal.loading')
                : currentCacheSize === 0
                  ? t('settings.cache.noCachedImages')
                  : formatBytes(currentCacheSize)}
            </span>
            <span className="storage-usage__limit">{limitLabel}</span>
          </div>
          <div className={`storage-usage__track${cacheEnabled ? '' : ' storage-usage__track--muted'}`}>
            <div
              className="storage-usage__fill storage-usage__fill--images"
              style={{ width: `${imagesPercent}%` }}
            />
            <div
              className="storage-usage__fill storage-usage__fill--assets"
              style={{ width: `${assetsPercent}%` }}
            />
          </div>
          {currentCacheSize > 0 && (
            <div className="storage-usage__legend">
              <span className="storage-legend">
                <span className="storage-legend__dot storage-legend__dot--images" />
                <span className="storage-legend__label">{t('settings.cache.legendImages')}</span>
                <span className="storage-legend__value">{formatBytes(breakdown.images)}</span>
              </span>
              <span className="storage-legend">
                <span className="storage-legend__dot storage-legend__dot--assets" />
                <span className="storage-legend__label">{t('settings.cache.legendData')}</span>
                <span className="storage-legend__value">{formatBytes(breakdown.assets)}</span>
              </span>
            </div>
          )}
          <div className="storage-usage__hint">
            {t('settings.cache.maxSizeDescription')}
          </div>
          </div>
        </div>

        {/* Cache controls grouped on a single glass card divided by hairlines */}
        <div className="settings-group">
          <h4 className="settings-group__title">{t('settings.cache.title')}</h4>
          <div className="settings-group__card">
            <div className="settings-row">
              <div className="settings-row__main">
                <div className="settings-row__icon">
                  <HardDrive size={20} />
                </div>
                <div className="settings-row__text">
                  <div className="settings-row__label">
                    {t('settings.cache.enable')}
                  </div>
                  <div className="settings-row__desc">
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

            <div className="settings-row">
              <div className="settings-row__main">
                <div className="settings-row__icon">
                  <Database size={20} />
                </div>
                <div className="settings-row__text">
                  <div className="settings-row__label">
                    {t('settings.cache.maxSize')}
                  </div>
                  <div className="settings-row__desc">
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

            <div className="settings-row">
              <div className="settings-row__main">
                <div className="settings-row__icon">
                  <Trash2 size={20} />
                </div>
                <div className="settings-row__text">
                  <div className="settings-row__label">
                    {t('settings.cache.clear')}
                  </div>
                  <div className="settings-row__desc">
                    {t('settings.cache.clearDescription')}
                  </div>
                </div>
              </div>
              <button
                className="btn btn-secondary btn-sm"
                onClick={handleClearCacheClick}
                disabled={isClearing || currentCacheSize === 0}
                aria-label={t('settings.cache.clear')}
              >
                {isClearing ? t('modal.loading') : t('settings.cache.clearButton')}
              </button>
            </div>

            <div
              className="settings-row settings-row--clickable"
              onClick={openCacheManager}
              role="button"
              tabIndex={0}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  openCacheManager();
                }
              }}
            >
              <div className="settings-row__main">
                <div className="settings-row__icon">
                  <FolderOpen size={20} />
                </div>
                <div className="settings-row__text">
                  <div className="settings-row__label">
                    {t('settings.cache.manage')}
                  </div>
                  <div className="settings-row__desc">
                    {t('settings.cache.manageDescription')}
                  </div>
                </div>
              </div>
              <div className="settings-row__arrow">
                <ChevronRight size={20} />
              </div>
            </div>
          </div>
        </div>

        <ConfirmationDialog
          isOpen={showClearConfirm}
          title={t('settings.cache.clear')}
          message={t('settings.cache.clearConfirm')}
          confirmText={t('common.confirm')}
          isDanger={true}
          onCancel={() => setShowClearConfirm(false)}
          onConfirm={handleClearCacheConfirm}
        />

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
