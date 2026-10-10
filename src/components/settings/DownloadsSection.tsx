// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useMemo, useRef, useState, type CSSProperties } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronRight, Trash2 } from 'lucide-react';
import { CACHE, EVENTS, SETTINGS, UI } from '../../config';
import {
  getCacheEnabled,
  setCacheEnabled,
  getCacheMaxSize,
  setCacheMaxSize,
  getForceOffline,
  setForceOffline,
} from '../../hooks/useSettings';
import { clearCache, getCacheBreakdown } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useSettingToggle } from '../../hooks/useSettingToggle';
import { useToasts } from '../../hooks/useToasts';
import type { CacheBreakdown } from '../../types';
import { formatBytes } from '../../utils';
import { ConfirmationDialog } from '../shared/ConfirmationDialog';
import { ErrorDisplay } from '../shared/ErrorDisplay';
import { CacheManagerModal } from './CacheManagerModal';
import { SettingsHero } from './SettingsHero';
import { SelectMenu, SettingsGroup, SettingsRow, ToggleRow, type SelectOption } from './controls';

/** Bar widths in percent of the limit; a non-empty category keeps a visible floor and the sum never overflows. */
function usagePercents({ images, assets }: CacheBreakdown, limit: number) {
  if (limit <= 0) return { images: 0, assets: 0 };
  const floor = (bytes: number) => (bytes > 0 ? Math.max((bytes / limit) * 100, UI.STORAGE_BAR_MIN_PERCENT) : 0);
  const assetsPct = Math.min(floor(assets), 100);
  return { images: Math.min(floor(images), 100 - assetsPct), assets: assetsPct };
}

function UsageBar({ breakdown, limit, muted }: { breakdown: CacheBreakdown; limit: number; muted: boolean }) {
  const { t } = useTranslation();
  const pct = usagePercents(breakdown, limit);
  const empty = breakdown.total === 0;
  return (
    <span className={`downloads-usage${muted ? ' is-muted' : ''}`}>
      <span className="downloads-usage__bar" aria-hidden="true">
        {pct.images > 0 && <i className="downloads-usage__seg downloads-usage__seg--images" style={{ '--w': `${pct.images}%` } as CSSProperties} />}
        {pct.assets > 0 && <i className="downloads-usage__seg downloads-usage__seg--assets" style={{ '--w': `${pct.assets}%` } as CSSProperties} />}
      </span>
      <span className="downloads-usage__legend">
        {empty ? (
          <span className="downloads-usage__key downloads-usage__key--empty">{t('settings.cache.noCachedImages')}</span>
        ) : (
          <>
            <span className="downloads-usage__key downloads-usage__key--images">
              {t('settings.cache.legendImages')} {formatBytes(breakdown.images)}
            </span>
            <span className="downloads-usage__key downloads-usage__key--assets">
              {t('settings.cache.legendData')} {formatBytes(breakdown.assets)}
            </span>
          </>
        )}
      </span>
    </span>
  );
}

export function DownloadsSection() {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();
  const cacheEnabled = useSettingToggle(getCacheEnabled, setCacheEnabled, {
    fallback: SETTINGS.DEFAULTS.CACHE_ENABLED,
    toastKey: 'settings.toast.cacheToggle',
  });
  const maxSize = useSettingToggle(getCacheMaxSize, setCacheMaxSize, {
    fallback: CACHE.DEFAULT_SIZE,
    toastKey: 'settings.toast.cacheSize',
  });
  const offline = useSettingToggle(getForceOffline, setForceOffline, {
    fallback: SETTINGS.DEFAULTS.FORCE_OFFLINE,
    toastKey: 'settings.toast.forceOffline',
  });
  const { data, loading, error, reload } = useAsyncData<CacheBreakdown>(getCacheBreakdown, []);
  const [confirmClear, setConfirmClear] = useState(false);
  const [clearing, setClearing] = useState(false);
  const [managerOpen, setManagerOpen] = useState(false);
  const managerRowRef = useRef<HTMLElement | null>(null);

  const breakdown = data ?? CACHE.EMPTY_BREAKDOWN;
  const limit = maxSize.value ?? CACHE.DEFAULT_SIZE;
  const enabled = cacheEnabled.value ?? SETTINGS.DEFAULTS.CACHE_ENABLED;
  const isOffline = offline.value ?? SETTINGS.DEFAULTS.FORCE_OFFLINE;
  const nothingCached = !loading && !error && breakdown.total === 0;

  const sizeOptions = useMemo<SelectOption<string>[]>(() => {
    const options = CACHE.SIZE_OPTIONS.map((o) => ({ value: String(o.value), label: o.label }));
    return options.some((o) => o.value === String(limit)) ? options : [...options, { value: String(limit), label: formatBytes(limit) }];
  }, [limit]);
  const limitLabel = sizeOptions.find((o) => o.value === String(limit))?.label ?? formatBytes(limit);

  const handleMaxSize = async (value: string) => {
    await maxSize.set(Number(value));
    reload();
  };

  // The nav sub-line reads cache usage on SETTINGS_CHANGED; clearing changes no setting
  const refreshUsage = () => {
    reload();
    window.dispatchEvent(new Event(EVENTS.SETTINGS_CHANGED));
  };

  const handleClear = async () => {
    setConfirmClear(false);
    setClearing(true);
    try {
      await clearCache();
      showSuccess(t('settings.toast.cacheClearSuccess'));
    } catch {
      showError(t('settings.toast.cacheClearError'));
    } finally {
      setClearing(false);
      refreshUsage();
    }
  };

  const usageDescription = loading
    ? t('modal.loading')
    : t('settings.nav.cacheUsed', { used: formatBytes(breakdown.total), limit: limitLabel });

  return (
    <>
      <SettingsHero
        hero="downloads"
        title={t('settings.nav.downloads')}
        lead={t('settings.downloads.lead')}
        heroState={{ offline: String(isOffline) }}
        loading={offline.value === undefined}
        actions={
          !nothingCached &&
          !loading && (
            <button type="button" className="btn btn-secondary btn-pill" onClick={() => setConfirmClear(true)} disabled={clearing}>
              <Trash2 size={16} aria-hidden="true" />
              {t('settings.cache.clear')}
            </button>
          )
        }
      />

      <SettingsGroup eyebrow={t('settings.downloads.cacheEyebrow')}>
        {error ? (
          <div className="downloads-usage-error">
            <ErrorDisplay error={error} onRetry={reload} compact />
          </div>
        ) : (
          <SettingsRow
            label={t('settings.cache.usageTitle')}
            description={usageDescription}
            control={<UsageBar breakdown={breakdown} limit={limit} muted={!enabled || loading} />}
          />
        )}
        <ToggleRow
          label={t('settings.cache.enable')}
          description={t('settings.cache.enableDescription')}
          checked={enabled}
          loading={cacheEnabled.value === undefined}
          busy={cacheEnabled.busy}
          onChange={cacheEnabled.set}
        />
        <SettingsRow
          label={t('settings.cache.maxSize')}
          description={t('settings.cache.maxSizeDescription')}
          control={
            <SelectMenu
              value={String(limit)}
              options={sizeOptions}
              onChange={handleMaxSize}
              ariaLabel={t('settings.cache.maxSize')}
              disabled={!enabled || maxSize.value === undefined || maxSize.busy}
            />
          }
        />
        <SettingsRow
          label={t('settings.cache.manage')}
          description={t('settings.cache.manageDescription')}
          onClick={(event) => {
            managerRowRef.current = event.currentTarget;
            setManagerOpen(true);
          }}
          control={nothingCached ? <span className="downloads-value">{t('settings.downloads.manageEmpty')}</span> : null}
          trailing={<ChevronRight size={16} aria-hidden="true" />}
        />
      </SettingsGroup>

      <SettingsGroup eyebrow={t('settings.downloads.networkEyebrow')}>
        <ToggleRow
          label={t('settings.forceOffline')}
          description={t('settings.forceOfflineDescription')}
          checked={isOffline}
          loading={offline.value === undefined}
          busy={offline.busy}
          onChange={offline.set}
        />
      </SettingsGroup>

      <ConfirmationDialog
        isOpen={confirmClear}
        title={t('settings.cache.clear')}
        message={t('settings.cache.clearConfirm')}
        confirmText={t('common.confirm')}
        isDanger
        onCancel={() => setConfirmClear(false)}
        onConfirm={handleClear}
      />

      <CacheManagerModal
        isOpen={managerOpen}
        returnFocusRef={managerRowRef}
        onClose={() => {
          setManagerOpen(false);
          refreshUsage();
        }}
      />
    </>
  );
}
