import { useState, useMemo } from 'react';
import {
  Package, Layers, Star, RefreshCw, AppWindow, Box,
  Download, ArrowRight, Monitor, Calendar, CircleCheck,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getImagesForBoard, listCachedImages } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useSkeletonLoading } from '../../hooks/useSkeletonLoading';
import {
  getOsInfo, getAppInfo, getKernelType, getImageVariantLabel,
  KERNEL_BADGES, UI,
  isTrunkImage, IMAGE_FILTER_PREDICATES, FILTER_BUTTONS, categoryOf, type OsCategory,
} from '../../config';
import { getMonoLogo } from '../../config/mono-logos';
import { formatFileSize, hexToRgba, staggerDelay, splitArmbianVersion, formatDate, armbianIdentityKey } from '../../utils';
import { distroGradient, distroBlock, distroVars } from '../../utils/distroTheme';
import { ErrorDisplay, ConfirmationDialog } from '../shared';
import type { BoardInfo, ImageInfo, ImageFilterType, CachedImageInfo } from '../../types';

/** Non-promoted filter keys available in the toolbar (recommended is pinned, not a filter). */
type RestFilter = Exclude<ImageFilterType, 'all' | 'recommended'>;

/** Toolbar filter buttons, excluding the pinned "recommended" entry. */
const REST_FILTER_BUTTONS = FILTER_BUTTONS.filter((b) => b.key !== 'recommended');

/** A tidy soft-tinted pill: low-alpha fill, colored text + dot, colored border. */
function SoftBadge({ color, label, dot = true }: { color: string; label: string; dot?: boolean }) {
  return (
    <span className="os-tag" style={{ color, background: hexToRgba(color, 0.14), borderColor: hexToRgba(color, 0.36) }}>
      {dot && <span className="os-tag__dot" style={{ background: color }} />}
      {label}
    </span>
  );
}

/** Discreet check marking an image already present in the local cache; a tooltip explains it on hover.
 * Use `light` over coloured gradient cards. */
function CachedBadge({ label, light = false }: { label: string; light?: boolean }) {
  return (
    <span className={`os-cached${light ? ' os-cached--light' : ''}`} aria-label={label}>
      <CircleCheck size={light ? 16 : 14} />
      <span className="os-cached__tip" role="tooltip">{label}</span>
    </span>
  );
}

/** Section header: a color-coded icon chip, title, count pill and a fading rule. */
function SectionHeader({
  icon: Icon,
  title,
  color,
  count,
  date,
}: {
  icon: typeof Layers;
  title: string;
  color: string;
  count: number;
  /** Pre-formatted build date shown at the right end of the header rule. */
  date?: string;
}) {
  return (
    <div className="os-section__head">
      <span className="os-section__icon" style={{ color, background: hexToRgba(color, 0.15) }}>
        <Icon size={16} />
      </span>
      <h3 className="os-section__title">{title}</h3>
      <span className="os-section__count">{count}</span>
      <span className="os-section__rule" style={{ background: `linear-gradient(90deg, ${hexToRgba(color, 0.3)}, transparent)` }} />
      {date && (
        <span className="os-section__date" title={date}>
          <Calendar size={12} />
          {date}
        </span>
      )}
    </div>
  );
}

interface OsPanelProps {
  board: BoardInfo;
  onSelect: (image: ImageInfo) => void;
}

/** Inline OS image browser: promoted images pinned on top as "Recommended" cards, the rest filtered by the toolbar. */
export function OsPanel({ board, onSelect }: OsPanelProps) {
  const { t, i18n } = useTranslation();

  /** Latest build date across a group of images, formatted for the active locale. */
  function formatBuildDate(images: ImageInfo[]): string | undefined {
    const dates = images.map((img) => img.build_date).filter((d): d is string => !!d);
    if (dates.length === 0) return undefined;
    // ISO 8601 strings sort lexicographically, so the max is the most recent build.
    const latest = dates.reduce((a, b) => (a > b ? a : b));
    return formatDate(latest, i18n.language);
  }
  const [filterType, setFilterType] = useState<'all' | RestFilter>('all');
  const [pendingImage, setPendingImage] = useState<ImageInfo | null>(null);
  const [showWarning, setShowWarning] = useState(false);

  const { data: allImages, loading, error, reload } = useAsyncData<ImageInfo[]>(
    () => getImagesForBoard(board.slug),
    [board.slug]
  );

  const ready = !!(allImages && allImages.length > 0);
  const { showSkeleton } = useSkeletonLoading(loading, ready);

  // Locally cached images, keyed by parsed identity so a remote image can be flagged as already downloaded.
  const { data: cachedImages } = useAsyncData<CachedImageInfo[]>(() => listCachedImages(), []);
  const cachedKeys = useMemo(() => {
    const keys = new Set<string>();
    for (const cached of cachedImages ?? []) {
      const key = armbianIdentityKey(cached.filename);
      if (key) keys.add(key);
    }
    return keys;
  }, [cachedImages]);

  /** Whether this remote image already exists in the local cache. Identity comes from `direct_url`
   * (the real Armbian filename); `file_url` is an API redirect that lacks the full name. */
  function isCached(image: ImageInfo): boolean {
    if (cachedKeys.size === 0) return false;
    const key = armbianIdentityKey(image.direct_url);
    return key !== null && cachedKeys.has(key);
  }

  // Promoted images, always shown on top regardless of the active filter.
  const recommended = useMemo(() => allImages?.filter((img) => img.promoted) ?? [], [allImages]);

  const availableFilters = useMemo(() => {
    const result = {} as Record<RestFilter, boolean>;
    REST_FILTER_BUTTONS.forEach(({ key }) => {
      result[key as RestFilter] = !!allImages?.some((img) => !img.promoted && IMAGE_FILTER_PREDICATES[key](img));
    });
    return result;
  }, [allImages]);

  // Non-promoted images matching the active filter.
  const rest = useMemo(() => {
    const base = allImages?.filter((img) => !img.promoted) ?? [];
    if (filterType === 'all') return base;
    return base.filter(IMAGE_FILTER_PREDICATES[filterType]);
  }, [allImages, filterType]);

  // Warn before community-tier boards or rolling (trunk) development builds.
  function handleClick(image: ImageInfo) {
    const needsWarning = board.support_tier === 'community' || isTrunkImage(image);
    if (!needsWarning) {
      onSelect(image);
      return;
    }
    setPendingImage(image);
    setShowWarning(true);
  }

  /** Status label + dot color for a build's stability (used on recommended cards). */
  function statusOf(image: ImageInfo): { label: string; color: string } {
    if (isTrunkImage(image)) return { label: t('modal.rolling'), color: '#3b82f6' };
    return { label: t('modal.stable'), color: '#10b981' };
  }

  /** Clean version: strip the "-trunk.NN" rolling suffix, keep the 26.x.y number. */
  function versionLabel(image: ImageInfo): string {
    return splitArmbianVersion(image.release).base;
  }

  /** Recommended (gradient) card. */
  function renderRecommended(image: ImageInfo, index: number) {
    const osInfo = getOsInfo(image.distro_release);
    const appInfo = getAppInfo(image.preinstalled_application);
    const display = appInfo || osInfo;
    const distroName = osInfo?.name || image.distro_release || '';
    const status = statusOf(image);
    const kernelType = getKernelType(image.kernel_branch);
    const kernelBadge = kernelType ? KERNEL_BADGES[kernelType] : null;
    // Use the same clean monochrome vector marks as the compact cards (whitened over the gradient).
    const monoLogo = getMonoLogo(image.distro_release, image.preinstalled_application);

    return (
      <button
        key={image.file_url}
        type="button"
        className="dl-card os-enter"
        style={{ background: distroGradient(distroName), animationDelay: staggerDelay(index) }}
        onClick={() => handleClick(image)}
      >
        {monoLogo && <img className="dl-card__watermark" src={monoLogo} alt="" aria-hidden="true" />}
        <div className="dl-card__top">
          <div className="dl-card__logo">
            {monoLogo ? <img src={monoLogo} alt={display?.name || distroName} /> : <Package size={28} color="#fff" />}
          </div>
          <span className="dl-card__status">
            <span className="dl-dot" style={{ background: status.color }} />
            {status.label}
          </span>
        </div>
        <div className="dl-card__bottom">
          <div className="dl-card__head">
            <span className="dl-card__title">Armbian {versionLabel(image)} {getImageVariantLabel(image, t)}</span>
            {distroName && <span className="dl-card__sub">{distroName}</span>}
          </div>
          <div className="dl-card__foot">
            {(kernelBadge || image.kernel_branch) && (
              <span className="dl-card__kernel">
                <span className="dl-dot" style={{ background: kernelBadge?.color ?? '#10b981' }} />
                {kernelBadge?.label ?? image.kernel_branch}{image.kernel_version ? ` ${image.kernel_version}` : ''}
              </span>
            )}
            {image.storage?.toLowerCase() === 'ufs' && (
              <span className="dl-card__kernel">
                <span className="dl-dot" style={{ background: '#f59e0b' }} />
                UFS
              </span>
            )}
            <span className="dl-card__footright">
              {isCached(image) && <CachedBadge label={t('modal.cachedTooltip')} light />}
              {!!image.file_size && (
                <span className="dl-card__size">
                  <Download size={15} />
                  {formatFileSize(image.file_size, t('common.unknown'))}
                </span>
              )}
            </span>
          </div>
        </div>
      </button>
    );
  }

  /** Gallery tile for non-recommended images: distro-washed header, clean version + variant, kernel/release pills. */
  function renderOsCard(image: ImageInfo, index: number) {
    const kernelType = getKernelType(image.kernel_branch);
    const osInfo = getOsInfo(image.distro_release);
    const appInfo = getAppInfo(image.preinstalled_application);
    const display = appInfo || osInfo;
    const distroName = osInfo?.name || image.distro_release || '';
    // Whitened raster logos lose their detail, so the side block uses a clean monochrome vector mark.
    const monoLogo = getMonoLogo(image.distro_release, image.preinstalled_application);
    const kernelBadge = kernelType ? KERNEL_BADGES[kernelType] : null;

    return (
      <button
        key={image.file_url}
        type="button"
        className="os-card os-enter"
        style={{ ...distroVars(distroName), animationDelay: staggerDelay(index) }}
        onClick={() => handleClick(image)}
      >
        <div className="os-card__side" style={{ background: distroBlock(distroName) }}>
          <div className="os-card__logo">
            {monoLogo ? <img src={monoLogo} alt={display?.name || distroName} /> : <Package size={34} color="#fff" />}
          </div>
        </div>
        <div className="os-card__body">
          <div className="os-card__info">
            <span className="os-card__title">Armbian {versionLabel(image)} {getImageVariantLabel(image, t)}</span>
            {distroName && <span className="os-card__sub">{distroName}</span>}
          </div>
          <div className="os-card__foot">
            <div className="os-card__badges">
              {kernelBadge && (
                <SoftBadge
                  color={kernelBadge.color}
                  label={`${kernelBadge.label}${image.kernel_version ? ` ${image.kernel_version}` : ''}`}
                />
              )}
              {image.storage?.toLowerCase() === 'ufs' && <SoftBadge color="#f59e0b" label="UFS" />}
            </div>
            <div className="os-card__footright">
              {isCached(image) && <CachedBadge label={t('modal.cachedTooltip')} />}
              {!!image.file_size && (
                <span className="os-card__size">
                  <Download size={12} />
                  {formatFileSize(image.file_size, t('common.unknown'))}
                </span>
              )}
              <ArrowRight className="os-card__arrow" size={18} />
            </div>
          </div>
        </div>
      </button>
    );
  }

  /** Render non-recommended images: flat grid under a specific filter, split into labelled sections under "All Images". */
  function renderRest() {
    if (filterType !== 'all') {
      return <div className="os-rest-grid">{rest.map(renderOsCard)}</div>;
    }
    const groups: Array<{ id: OsCategory; title: string; icon: typeof Layers; color: string }> = [
      { id: 'desktop', title: t('modal.groupDesktop'), icon: Monitor, color: '#3b82f6' },
      { id: 'minimal', title: t('modal.groupMinimal'), icon: Box, color: '#14b8a6' },
      { id: 'apps', title: t('modal.groupApps'), icon: AppWindow, color: '#8b5cf6' },
      { id: 'rolling', title: t('modal.groupRolling'), icon: RefreshCw, color: '#f59e0b' },
    ];
    return groups.map(({ id, title, icon, color }) => {
      const items = rest.filter((img) => categoryOf(img) === id);
      if (items.length === 0) return null;
      return (
        <section key={id} className="os-section">
          <SectionHeader icon={icon} title={title} color={color} count={items.length} date={formatBuildDate(items)} />
          <div className="os-rest-grid">{items.map(renderOsCard)}</div>
        </section>
      );
    });
  }

  return (
    <div className="mfr-panel">
      <div className="mfr-panel__head os-filters">
        <button className={`filter-btn ${filterType === 'all' ? 'active' : ''}`} onClick={() => setFilterType('all')}>
          <Layers size={14} />
          {t('modal.allImages')}
        </button>
        {REST_FILTER_BUTTONS.map(({ key, labelKey, icon: Icon }) =>
          availableFilters[key as RestFilter] ? (
            <button
              key={key}
              className={`filter-btn ${filterType === key ? 'active' : ''}`}
              onClick={() => setFilterType(key as RestFilter)}
            >
              <Icon size={14} />
              {t(labelKey)}
            </button>
          ) : null
        )}
      </div>

      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : showSkeleton ? (
        <div className="os-body">
          <div className="os-rest-grid">
            {Array.from({ length: UI.SKELETON.OS_PANEL }).map((_, i) => (
              <div key={i} className="os-card is-skeleton">
                <div className="os-card__side">
                  <span className="sk-shim os-sk-logo" />
                </div>
                <div className="os-card__body">
                  <div className="os-card__info">
                    <span className="sk-shim os-sk-title" />
                    <span className="sk-shim os-sk-sub" />
                  </div>
                  <div className="os-card__foot">
                    <span className="sk-shim os-sk-badge" />
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      ) : recommended.length === 0 && rest.length === 0 ? (
        <div className="mfr-empty">{t('modal.noImages')}</div>
      ) : (
        <div className="os-body">
          {recommended.length > 0 && (
            <section className="os-section">
              <SectionHeader icon={Star} title={t('modal.promoted')} color="#f2651f" count={recommended.length} date={formatBuildDate(recommended)} />
              <div
                className="os-rec-grid"
                style={{ gridTemplateColumns: `repeat(${Math.min(recommended.length, 3)}, minmax(0, 1fr))` }}
              >
                {recommended.map(renderRecommended)}
              </div>
            </section>
          )}
          {rest.length > 0 && renderRest()}
        </div>
      )}

      {showWarning && pendingImage && (
        <ConfirmationDialog
          isOpen={showWarning}
          title={t('modal.imageStatusTitle')}
          message={
            board.support_tier === 'community'
              ? t('modal.communityBoardMessage')
              : t('modal.rollingBuildMessage')
          }
          confirmText={t('common.confirm')}
          cancelText={t('common.cancel')}
          isDanger={false}
          onCancel={() => {
            setPendingImage(null);
            setShowWarning(false);
          }}
          onConfirm={() => {
            if (pendingImage) onSelect(pendingImage);
            setPendingImage(null);
            setShowWarning(false);
          }}
        />
      )}
    </div>
  );
}
