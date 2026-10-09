// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useRef, type ReactNode, type RefObject, type SyntheticEvent } from 'react';
import { Trans, useTranslation } from 'react-i18next';
import { Check, ChevronLeft, ChevronRight, RefreshCw, Search, SearchX, X } from 'lucide-react';
import { SUPPORT_TIER_LABEL, UI } from '../../config';
import type { BoardPickerState } from '../../hooks/useBoardPicker';
import { useBoardPhotos } from '../../hooks/useBoardPhotos';
import { useFitCount } from '../../hooks/useFitCount';
import type { BoardInfo } from '../../types';
import { stripVendorPrefix } from '../../utils';
import { opaqueFitTransform } from '../../utils/imageFit';
import { VendorLogo } from './VendorLogo';
import { BoardImage, EmptyState, ErrorDisplay, GridPager, HighlightText, PageSwap } from '../shared';

const SHEET = UI.BOARD_SHEET;

// Applied straight to the element: the transform must land before the first paint of the loaded photo
function fitPhoto(event: SyntheticEvent<HTMLImageElement>) {
  const img = event.currentTarget;
  const transform = opaqueFitTransform(img, SHEET.PHOTO_FILL);
  if (transform) img.style.transform = transform;
  img.dataset.fit = 'true';
}

/** A board photo that fills its well by its visible content; a shimmer while it loads. */
export function SheetPhoto({ src, className = 'bps-card__img' }: { src: string | null | undefined; className?: string }) {
  return (
    <span className={className}>
      {src === undefined ? <span className="sk-shim bps-card__shim" /> : <BoardImage src={src} alt="" onLoad={fitPhoto} />}
    </span>
  );
}

interface SheetCardProps {
  slug: string;
  name: string;
  board: BoardInfo | null;
  photo: string | null | undefined;
  needle?: string;
  on?: boolean;
  onToggle?: () => void;
  action?: ReactNode;
  className?: string;
}

/** A board card of the sheet: photo well, brand eyebrow beside the support tier pill, and the name. */
export function SheetCard({ slug, name, board, photo, needle = '', on = false, onToggle, action, className = '' }: SheetCardProps) {
  const vendor = board?.vendor_name ?? '';
  const tier = board?.support_tier ?? '';
  const tierLabel = SUPPORT_TIER_LABEL[tier];
  const content = (
    <>
      <SheetPhoto src={photo} />
      <span className="bps-card__info">
        <span className="bps-card__top">
          <span className="bps-card__vendor">
            <HighlightText text={vendor} needle={needle} />
          </span>
          {tierLabel && <span className={`bp-tier is-${tier}`}>{tierLabel}</span>}
        </span>
        <span className="bps-card__name">
          <HighlightText text={stripVendorPrefix(name, vendor)} needle={needle} />
        </span>
      </span>
    </>
  );
  if (onToggle) {
    return (
      <button type="button" className={`bps-card${on ? ' is-on' : ''} ${className}`} aria-pressed={on} data-slug={slug} onClick={onToggle}>
        {content}
        <span className="bps-card__tick" aria-hidden="true">
          <Check size={13} strokeWidth={3.2} />
        </span>
      </button>
    );
  }
  return (
    <div className={`bps-card is-static ${className}`} role="listitem" data-slug={slug}>
      {content}
      {action}
    </div>
  );
}

export function SheetCardSkeletons({ count }: { count: number }) {
  return (
    <>
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="bps-card is-skeleton" aria-hidden="true">
          <span className="bps-card__img">
            <span className="sk-shim bps-card__shim" />
          </span>
          <span className="bps-card__info">
            <span className="sk-shim bps-card__sk-vendor" />
            <span className="sk-shim bps-card__sk-name" />
          </span>
        </div>
      ))}
    </>
  );
}

interface BoardSearchProps {
  picker: BoardPickerState;
  inputRef: RefObject<HTMLInputElement | null>;
}

/** The sheet's search field: boards and brands; Enter opens the brand when exactly one matches. */
export function BoardSearch({ picker, inputRef }: BoardSearchProps) {
  const { t } = useTranslation();
  const { query, setQuery, brandHits, pickVendor } = picker;
  return (
    <label className={`bps-search${query ? ' has-query' : ''}`}>
      <Search size={15} aria-hidden="true" />
      <input
        ref={inputRef}
        type="text"
        value={query}
        placeholder={t('settings.autoconfig.boardSheet.searchPlaceholder')}
        aria-label={t('settings.autoconfig.boardSheet.searchPlaceholder')}
        autoFocus
        spellCheck={false}
        autoComplete="off"
        onChange={(e) => setQuery(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Escape' && query) {
            e.preventDefault();
            setQuery('');
          } else if (e.key === 'Enter' && picker.needle && brandHits.length === 1) {
            e.preventDefault();
            pickVendor(brandHits[0].id);
          }
        }}
      />
      {query ? (
        <button
          type="button"
          className="bps-search__clear"
          aria-label={t('modal.clearSearch')}
          onClick={() => {
            setQuery('');
            inputRef.current?.focus();
          }}
        >
          <X size={11} strokeWidth={3} aria-hidden="true" />
        </button>
      ) : (
        <kbd className="bps-kbd" aria-hidden="true">
          {SHEET.SEARCH_KEY}
        </kbd>
      )}
    </label>
  );
}

interface BrowseBarProps {
  picker: BoardPickerState;
  onMoreBrands: () => void;
  menu: ReactNode;
}

/** Brands matching the search: as many whole chips as the row holds, the rest behind "+N more". */
function BrandHits({ picker, onMoreBrands, menu }: BrowseBarProps) {
  const { t } = useTranslation();
  const { needle, brandHits } = picker;
  const rowRef = useRef<HTMLDivElement>(null);
  const copyRef = useRef<HTMLDivElement>(null);
  const candidates = brandHits.slice(0, SHEET.BRAND_HITS);
  const label = t('settings.autoconfig.boardSheet.brandsLabel');
  const fit = useFitCount(
    rowRef,
    copyRef,
    brandHits.length > candidates.length,
    `${label}\x1f${t('settings.autoconfig.boardSheet.moreBrands', { count: brandHits.length })}\x1f${candidates.map((v) => v.id).join('\x1f')}`
  );
  const hits = candidates.slice(0, fit);
  const extra = brandHits.length - hits.length;

  return (
    <div ref={rowRef} className="bps-bar__hits">
      <span className="bps-bar__label">{label}</span>
      {brandHits.length === 0 && <span className="bps-bar__none">{t('settings.autoconfig.boardSheet.noBrandMatch')}</span>}
      {hits.map((v) => (
        <button key={v.id} type="button" className="bps-hit" onClick={() => picker.pickVendor(v.id)}>
          <VendorLogo name={v.name} src={v.logo} size="xs" />
          <span className="bps-hit__name">
            <HighlightText text={v.name} needle={needle} />
          </span>
          <span className="bps-hit__count">{v.boards.length}</span>
        </button>
      ))}
      {extra > 0 && (
        <button type="button" className="bps-hit is-more" aria-haspopup="dialog" aria-expanded={!!menu} onClick={onMoreBrands}>
          {t('settings.autoconfig.boardSheet.moreBrands', { count: extra })}
        </button>
      )}
      <div ref={copyRef} className="bps-bar__hits bps-bar__measure" aria-hidden="true">
        <span className="bps-bar__label">{label}</span>
        {candidates.map((v) => (
          <span key={v.id} className="bps-hit">
            <span className="vlogo vlogo--xs" />
            <span className="bps-hit__name">
              <HighlightText text={v.name} needle={needle} />
            </span>
            <span className="bps-hit__count">{v.boards.length}</span>
          </span>
        ))}
        <span className="bps-hit is-more">{t('settings.autoconfig.boardSheet.moreBrands', { count: brandHits.length })}</span>
      </div>
    </div>
  );
}

/** Context row under the header: matching brands while searching, the picked brand, or the catalogue size. */
export function BrowseBar({ picker, onMoreBrands, menu }: BrowseBarProps) {
  const { t } = useTranslation();
  const { needle, shown, vendor, picked, boards } = picker;
  if (!boards) return <div className="bps-bar" />;

  if (needle) {
    return (
      <div className="bps-bar">
        <BrandHits picker={picker} onMoreBrands={onMoreBrands} menu={menu} />
        <span className="bps-bar__info">
          <Trans i18nKey="settings.autoconfig.boardSheet.matchCount" count={shown.length} components={{ b: <b /> }} />
        </span>
        {menu}
      </div>
    );
  }

  if (vendor) {
    const on = shown.filter((b) => picked.has(b.slug)).length;
    const all = on === shown.length;
    return (
      <div className="bps-bar">
        <span className="bps-hit is-set">
          <VendorLogo name={vendor.name} src={vendor.logo} size="xs" />
          <span className="bps-hit__name">{vendor.name}</span>
          <button
            type="button"
            className="bps-hit__clear"
            aria-label={t('settings.autoconfig.boardSheet.showAllBrands')}
            title={t('settings.autoconfig.boardSheet.showAllBrands')}
            onClick={() => picker.pickVendor('')}
          >
            <X size={12} strokeWidth={2.8} aria-hidden="true" />
          </button>
        </span>
        <span className="bps-bar__info">
          <Trans
            i18nKey="settings.autoconfig.boardSheet.brandTally"
            count={shown.length}
            values={{ selected: on }}
            components={{ b: <b /> }}
          />
        </span>
        {shown.length > 0 && (
          <button type="button" className="bps-link" onClick={() => picker.setMany(shown, !all)}>
            {all
              ? t('settings.autoconfig.boardSheet.deselectAll')
              : t('settings.autoconfig.boardSheet.selectAll', { count: shown.length })}
          </button>
        )}
      </div>
    );
  }

  return (
    <div className="bps-bar">
      <span className="bps-bar__info">
        <Trans
          i18nKey="settings.autoconfig.boardSheet.catalogSummary"
          count={shown.length}
          values={{ brands: picker.vendorCount }}
          components={{ b: <b /> }}
        />
      </span>
    </div>
  );
}

/** The paged card grid with its loading, error and empty states and the floating page pill. */
export function BrowseGrid({ picker, onClearSearch }: { picker: BoardPickerState; onClearSearch: () => void }) {
  const { t } = useTranslation();
  const { error, reload, showSkeleton, boards, shown, needle, vendor, pageItems, picked, photos } = picker;

  let body;
  if (error) {
    body = <ErrorDisplay error={error} onRetry={reload} compact />;
  } else if (showSkeleton || !boards) {
    body = (
      <div className="bps__grid">
        <SheetCardSkeletons count={SHEET.PER_PAGE} />
      </div>
    );
  } else if (shown.length === 0) {
    body = needle ? (
      <EmptyState
        icon={SearchX}
        title={t('settings.autoconfig.boardSheet.noMatchTitle', { query: picker.query.trim() })}
        hint={
          vendor
            ? t('settings.autoconfig.boardSheet.noMatchInBrand', { brand: vendor.name })
            : t('settings.autoconfig.boardSheet.noMatchHint')
        }
        action={{ label: t('modal.clearSearch'), icon: X, onClick: onClearSearch }}
      />
    ) : (
      <EmptyState
        icon={SearchX}
        title={t('modal.noBoards')}
        hint={t('modal.listEmptyHint')}
        action={{ label: t('device.refresh'), icon: RefreshCw, onClick: reload }}
      />
    );
  } else {
    body = (
      <PageSwap
        swapKey={pageItems.map((b) => b.slug).join('\x1f')}
        motion={picker.motion}
        className="bps__grid"
        role="group"
        aria-label={t('settings.autoconfig.editor.chooseBoards')}
      >
        {pageItems.map((b) => (
          <SheetCard
            key={b.slug}
            slug={b.slug}
            name={b.name}
            board={b}
            photo={photos[b.slug]}
            needle={needle}
            on={picked.has(b.slug)}
            onToggle={() => picker.toggle(b)}
          />
        ))}
      </PageSwap>
    );
  }

  return (
    <div className="bps-stage">
      {body}
      {!error && (
        <GridPager
          compact
          pageCount={picker.pageCount}
          page={picker.page}
          onChange={picker.setPage}
          ariaLabel={t('settings.autoconfig.editor.boardPages')}
        />
      )}
    </div>
  );
}

/** Footer ledger: the picked boards as a stack of photos with the count; opens or leaves the review. */
export function BoardLedger({ picker }: { picker: BoardPickerState }) {
  const { t } = useTranslation();
  const { picks, view, setView } = picker;
  const count = picks.length;
  const latest = [...picks].reverse();
  const thumbs = latest.slice(0, count > SHEET.LEDGER_THUMBS ? SHEET.LEDGER_THUMBS - 1 : SHEET.LEDGER_THUMBS);
  const photos = useBoardPhotos(thumbs.map((p) => p.slug));
  const review = view === 'review';

  return (
    <button
      type="button"
      className="bps-ledger"
      disabled={count === 0 && !review}
      onClick={() => setView(review ? 'browse' : 'review')}
    >
      {count > 0 && (
        <span className="bps-ledger__stack" aria-hidden="true">
          {thumbs.map((p) => (
            <SheetPhoto key={p.slug} src={photos[p.slug]} className="bps-ledger__thumb" />
          ))}
          {count > thumbs.length && <span className="bps-ledger__more">+{count - thumbs.length}</span>}
        </span>
      )}
      <span className="bps-ledger__text">
        <span className="bps-ledger__count" aria-live="polite">
          {count
            ? t('settings.autoconfig.boardSheet.selectedBoards', { count })
            : t('settings.autoconfig.boardSheet.noneSelected')}
        </span>
        {count === 0 ? (
          <span className="bps-ledger__link is-warn">{t('settings.autoconfig.editor.saveNeedsBoard')}</span>
        ) : review ? (
          <span className="bps-ledger__link">
            <ChevronLeft size={12} strokeWidth={2.6} aria-hidden="true" />
            {t('settings.autoconfig.boardSheet.backToAll')}
          </span>
        ) : (
          <span className="bps-ledger__link">
            {t('settings.autoconfig.boardSheet.showSelection')}
            <ChevronRight size={12} strokeWidth={2.6} aria-hidden="true" />
          </span>
        )}
      </span>
    </button>
  );
}
