import { useState, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { getBoards, getVendors } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useManufacturerList, type ManufacturerData } from '../../hooks/useVendorLogos';
import { useSkeletonLoading } from '../../hooks/useSkeletonLoading';
import { usePagedGrid } from '../../hooks/usePagedGrid';
import { ErrorDisplay, SearchBox, GridPager } from '../shared';
import { DEFAULT_COLOR, staggerDelay } from '../../utils';
import { VENDOR, UI, PARTNER_TIER_RANK } from '../../config';
import type { BoardInfo, VendorInfo, Manufacturer } from '../../types';

/** Approximate vendor card row height (min-height 206 + row gap) for the fit calc. */
const CARD_ROW = 224;

function MfrLogo({ manufacturer }: { manufacturer: ManufacturerData }) {
  if (!manufacturer.logo || manufacturer.id === VENDOR.FALLBACK_ID) {
    return <div className="mfr-card__initials">{manufacturer.name.substring(0, 2).toUpperCase()}</div>;
  }
  return <img className="mfr-card__logo" src={manufacturer.logo} alt={manufacturer.name} />;
}

interface ManufacturerPanelProps {
  onSelect: (manufacturer: Manufacturer) => void;
}

/** Inline manufacturer browser: grid of vendor logos with board counts and partner-tier badges. */
export function ManufacturerPanel({ onSelect }: ManufacturerPanelProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');

  const { data: boards, loading, error, reload } = useAsyncData<BoardInfo[]>(() => getBoards(), []);
  const { data: vendors } = useAsyncData<VendorInfo[]>(() => getVendors(), []);
  const { manufacturers, isLoaded } = useManufacturerList(boards, true, search);

  const ready = !!(manufacturers && manufacturers.length > 0 && isLoaded);
  const { showSkeleton } = useSkeletonLoading(loading, ready);

  // Map vendor slug -> partner tier (platinum/gold/silver) for badges.
  const tierMap = useMemo(() => {
    const map = new Map<string, string>();
    vendors?.forEach((v) => {
      const tier = v.partner_tier?.toLowerCase();
      if (tier && tier in PARTNER_TIER_RANK) map.set(v.slug, tier);
    });
    return map;
  }, [vendors]);

  // Partner-tier vendors first (platinum > gold > silver), keeping board-count order within each.
  const sorted = useMemo(() => {
    const rank = (id: string) => {
      const tier = tierMap.get(id);
      return tier !== undefined ? PARTNER_TIER_RANK[tier] : 99;
    };
    return manufacturers
      .map((m, i) => ({ m, i }))
      .sort((a, b) => rank(a.m.id) - rank(b.m.id) || a.i - b.i)
      .map((x) => x.m);
  }, [manufacturers, tierMap]);

  // Window-adaptive pagination (resets on search change). Logos are light/preloaded, so a high cap (120)
  // lets the page fill the screen without cutting a screenful in half on large monitors.
  const { setPage, pageCount, safePage, pagedItems: pagedMfrs, measureGrid } = usePagedGrid(
    sorted,
    CARD_ROW,
    search,
    120
  );

  return (
    <div className="mfr-panel">
      <div className="mfr-panel__head">
        <SearchBox value={search} onChange={setSearch} placeholder={t('modal.searchManufacturer')} autoFocus={false} />
      </div>

      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : showSkeleton ? (
        <div className="mfr-grid">
          {Array.from({ length: UI.SKELETON.MANUFACTURER_PANEL }).map((_, i) => (
            <div key={i} className="mfr-card is-skeleton">
              <div className="mfr-card__head">
                <span className="sk-shim mfr-sk-logo" />
              </div>
              <div className="mfr-card__body">
                <span className="sk-shim mfr-sk-name" />
                <span className="sk-shim mfr-sk-count" />
              </div>
            </div>
          ))}
        </div>
      ) : manufacturers.length === 0 ? (
        <div className="mfr-empty">{t('modal.noManufacturers')}</div>
      ) : (
        <>
          <div ref={measureGrid} className={`mfr-grid${pageCount > 1 ? ' mfr-grid--paged' : ''}`}>
            {pagedMfrs.map((mfr, index) => {
              const tier = tierMap.get(mfr.id);
              return (
                <button
                  key={mfr.id}
                  type="button"
                  className="mfr-card mfr-card--enter"
                  style={{ animationDelay: staggerDelay(index) }}
                  onClick={() => onSelect({ id: mfr.id, name: mfr.name, color: DEFAULT_COLOR, boardCount: mfr.boardCount })}
                >
                  <div className="mfr-card__head">
                    <MfrLogo manufacturer={mfr} />
                    {tier && <span className={`mfr-tier is-${tier}`}>{tier}</span>}
                  </div>
                  <div className="mfr-card__body">
                    <span className="mfr-card__name">{mfr.name}</span>
                    <span className="mfr-card__count">{t('home.boardCount', { count: mfr.boardCount })}</span>
                  </div>
                </button>
              );
            })}
          </div>

          <GridPager pageCount={pageCount} page={safePage} onChange={setPage} />
        </>
      )}
    </div>
  );
}
