import { useState, useEffect, useMemo, useRef } from 'react';
import { ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getBoards, getCachedBoardImage } from '../../hooks/useTauri';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useVendorLogos } from '../../hooks/useVendorLogos';
import { useSkeletonLoading } from '../../hooks/useSkeletonLoading';
import { usePagedGrid } from '../../hooks/usePagedGrid';
import { compareBoardsBySupport, staggerDelay, stripVendorPrefix } from '../../utils';
import { UI, SUPPORT_TIER_LABEL } from '../../config';
import { ErrorDisplay, SearchBox, BoardImage, GridPager, MarqueeText } from '../shared';
import type { BoardInfo, Manufacturer } from '../../types';

interface BoardPanelProps {
  manufacturer: Manufacturer;
  onSelect: (board: BoardInfo) => void;
}

/** Approximate board card row height (min-height 250 + row gap) for the fit calc. */
const CARD_ROW = 268;

/** Inline board browser: cards with board photo, support-tier badge, vendor label and name. */
export function BoardPanel({ manufacturer, onSelect }: BoardPanelProps) {
  const { t } = useTranslation();
  const [search, setSearch] = useState('');
  const [boardImages, setBoardImages] = useState<Record<string, string | null>>({});
  const loadedSlugsRef = useRef<Set<string>>(new Set());

  const { data: boards, loading, error, reload } = useAsyncData<BoardInfo[]>(() => getBoards(), []);
  const { isLoaded: vendorLogosChecked, getEffectiveVendor } = useVendorLogos(boards, true);

  const ready = !!(boards && boards.length > 0 && vendorLogosChecked);
  const { showSkeleton } = useSkeletonLoading(loading, ready);

  // Boards for the selected manufacturer, searched and sorted by support tier.
  const filteredBoards = useMemo(() => {
    if (!boards || !vendorLogosChecked) return [];
    const searchLower = search.toLowerCase();
    return boards
      .filter((board) => {
        if (getEffectiveVendor(board) !== manufacturer.id) return false;
        return (
          board.name.toLowerCase().includes(searchLower) ||
          board.slug.toLowerCase().includes(searchLower)
        );
      })
      .sort(compareBoardsBySupport);
  }, [boards, vendorLogosChecked, getEffectiveVendor, manufacturer.id, search]);

  // Window-adaptive pagination (resets when the manufacturer or search changes).
  const { setPage, pageCount, safePage, pagedItems: pagedBoards, measureGrid } = usePagedGrid(
    filteredBoards,
    CARD_ROW,
    `${manufacturer.id}\x1f${search}`
  );

  // Preload only the current page's photos, marking each slug loaded so it isn't fetched twice.
  useEffect(() => {
    if (pagedBoards.length === 0) return;
    const loadImages = async () => {
      await Promise.all(
        pagedBoards.map(async (board) => {
          if (loadedSlugsRef.current.has(board.slug)) return;
          const dataUri = await getCachedBoardImage(board.slug);
          loadedSlugsRef.current.add(board.slug);
          setBoardImages((prev) => ({ ...prev, [board.slug]: dataUri ?? null }));
        })
      );
    };
    loadImages();
  }, [pagedBoards]);

  return (
    <div className="mfr-panel">
      <div className="mfr-panel__head">
        <SearchBox value={search} onChange={setSearch} placeholder={t('modal.searchBoard')} autoFocus={false} />
      </div>

      {error ? (
        <ErrorDisplay error={error} onRetry={reload} compact />
      ) : showSkeleton ? (
        <div className="mfr-grid">
          {Array.from({ length: UI.SKELETON.BOARD_PANEL }).map((_, i) => (
            <div key={i} className="board-card is-skeleton">
              <div className="board-card__img">
                <span className="sk-shim board-card__imgskeleton" />
              </div>
              <div className="board-card__info">
                <span className="sk-shim board-sk-vendor" />
                <span className="sk-shim board-sk-name" />
              </div>
            </div>
          ))}
        </div>
      ) : filteredBoards.length === 0 ? (
        <div className="mfr-empty">{t('modal.noBoards')}</div>
      ) : (
        <>
          <div ref={measureGrid} className={`mfr-grid${pageCount > 1 ? ' mfr-grid--paged' : ''}`}>
            {pagedBoards.map((board, index) => {
              const tier = board.support_tier;
              const tierLabel = SUPPORT_TIER_LABEL[tier];
              const vendorName = board.vendor_name || '';
              // Strip a leading vendor name (e.g. "Radxa CM4-IO") so the kicker and name don't duplicate it.
              const displayName = stripVendorPrefix(board.name, vendorName);
              const showVendor = !!vendorName;
              const img = board.slug in boardImages ? boardImages[board.slug] : undefined;
              // A QDL board whose storage this build has no write path for can't be
              // flashed here; block it at selection instead of failing mid-flash.
              const needsUpdate = !!board.qdl && !board.qdl.supported;

              return (
                <button
                  key={board.slug}
                  type="button"
                  className={`board-card board-card--enter${needsUpdate ? ' board-card--locked' : ''}`}
                  style={{ animationDelay: staggerDelay(index) }}
                  onClick={() => onSelect(board)}
                  disabled={needsUpdate}
                  title={needsUpdate ? t('modal.boardUpdateRequiredHint') : undefined}
                >
                  <div className="board-card__img">
                    {img === undefined ? (
                      <div className="skeleton board-card__imgskeleton" />
                    ) : (
                      <BoardImage src={img} alt={board.name} />
                    )}
                    {tierLabel && !needsUpdate && (
                      <span className={`bp-tier is-${tier}`}>{tierLabel}</span>
                    )}
                    {needsUpdate && <span className="bp-lock">{t('modal.boardUpdateRequired')}</span>}
                  </div>
                  <div className="board-card__info">
                    {showVendor && <span className="board-card__vendor">{vendorName}</span>}
                    <div className="board-card__namerow">
                      <MarqueeText text={displayName} className="board-card__name" />
                      <ArrowRight className="board-card__arrow" size={18} />
                    </div>
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
