// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDown, ArrowUp, Check, CornerDownLeft, LayoutGrid, Search, SearchX, X } from 'lucide-react';
import { UI } from '../../config';
import { rankVendors, type BoardPickerState, type BoardVendor } from '../../hooks/useBoardPicker';
import { useBoardPhotos } from '../../hooks/useBoardPhotos';
import { useListboxKeys } from '../../hooks/useListboxKeys';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import { HighlightText } from '../shared';
import { SheetPhoto } from './BoardPicker';
import { VendorLogo } from './VendorLogo';

const SHEET = UI.BOARD_SHEET;

type MenuOption = { key: string; vendor: BoardVendor | null };
interface MenuSection {
  id: string;
  label: string;
  note: string;
  options: MenuOption[];
}

interface BrandMenuProps {
  picker: BoardPickerState;
  initialFilter: string;
  onClose: () => void;
  onSearchBoards: (query: string) => void;
}

/** Command menu of brands: a filter field, Recent, Partners and All brands, with a peek at the active brand's boards. */
export function BrandMenu({ picker, initialFilter, onClose, onSearchBoards }: BrandMenuProps) {
  const { t } = useTranslation();
  const listId = useId();
  const [filter, setFilter] = useState(initialFilter);
  const { isExiting, handleClose } = useModalExitAnimation({ onClose, duration: SHEET.MENU_CLOSE_MS });
  const rootRef = useRef<HTMLDivElement>(null);
  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const pointer = useRef('');

  const { vendors, partners, recent } = picker;
  // As it was on open, so the list does not reshuffle while the menu folds away after a pick
  const [current] = useState(picker.vendor);
  const needle = filter.trim().toLowerCase();

  const sections = useMemo<MenuSection[]>(() => {
    const options = (id: string, list: BoardVendor[]) => list.map((v) => ({ key: `${id}:${v.id}`, vendor: v }));
    if (needle) {
      const hits = rankVendors(vendors, needle);
      if (!hits.length) return [];
      return [
        {
          id: 'q',
          label: t(hits.length === 1 ? 'settings.autoconfig.boardSheet.bestMatch' : 'settings.autoconfig.boardSheet.matches'),
          note: t('settings.autoconfig.boardSheet.matchesOf', { count: hits.length, total: vendors.length }),
          options: options('q', hits),
        },
      ];
    }
    const out: MenuSection[] = [];
    if (current) out.push({ id: 'reset', label: '', note: '', options: [{ key: 'reset', vendor: null }] });
    if (recent.length) {
      out.push({
        id: 'recent',
        label: t('settings.autoconfig.boardSheet.recent'),
        note: t('settings.autoconfig.boardSheet.recentNote'),
        options: options('recent', recent),
      });
    }
    if (partners.length) {
      out.push({
        id: 'partners',
        label: t('settings.autoconfig.boardSheet.partners'),
        note: t('settings.autoconfig.boardSheet.partnersNote'),
        options: options('partners', partners),
      });
    }
    out.push({
      id: 'all',
      label: t('settings.autoconfig.boardSheet.allBrands'),
      note: String(vendors.length),
      options: options('all', vendors),
    });
    return out;
  }, [needle, vendors, partners, recent, current, t]);
  const flat = useMemo(() => sections.flatMap((s) => s.options), [sections]);
  const starts = useMemo(() => sections.map((_, i) => sections.slice(0, i).reduce((n, s) => n + s.options.length, 0)), [sections]);

  const pick = (option: MenuOption | undefined) => {
    if (!option) return;
    picker.pickVendor(option.vendor?.id ?? '');
    handleClose();
  };
  const initial = current && !initialFilter.trim() ? Math.max(0, flat.findIndex((o) => o.key === `all:${current.id}`)) : 0;
  const keys = useListboxKeys(flat.length, (i) => pick(flat[i]), handleClose, initial);
  const active = flat[keys.active];

  useEffect(() => {
    inputRef.current?.focus();
    inputRef.current?.select();
  }, []);

  useEffect(() => {
    const onDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (target && !rootRef.current?.contains(target)) handleClose();
    };
    window.addEventListener('pointerdown', onDown, true);
    return () => window.removeEventListener('pointerdown', onDown, true);
  }, [handleClose]);

  useEffect(() => {
    const scroller = scrollRef.current;
    if (!scroller) return;
    if (keys.active <= 0) {
      scroller.scrollTop = 0;
      return;
    }
    scroller.querySelector('[aria-selected="true"]')?.scrollIntoView({ block: 'nearest' });
  }, [keys.active, sections]);

  const peek = active?.vendor?.boards.slice(0, SHEET.PEEK_PHOTOS) ?? [];
  const peekPhotos = useBoardPhotos(peek.map((b) => b.slug));
  const optionId = (key: string) => `${listId}-${key.replace(/[^\w-]/g, '_')}`;

  const row = (option: MenuOption, index: number) => {
    const v = option.vendor;
    const isActive = index === keys.active;
    const common = {
      id: optionId(option.key),
      role: 'option' as const,
      'aria-selected': isActive,
      onClick: () => pick(option),
      onPointerMove: (event: React.PointerEvent) => {
        const at = `${event.clientX},${event.clientY}`;
        if (at === pointer.current) return;
        pointer.current = at;
        if (!isActive) keys.setActive(index);
      },
    };
    if (!v) {
      return (
        <div key={option.key} className={`bps-menu__row${isActive ? ' is-active' : ''}`} {...common}>
          <span className="vlogo vlogo--md is-glyph" aria-hidden="true">
            <LayoutGrid size={16} />
          </span>
          <span className="bps-menu__main">
            <span className="bps-menu__name">{t('settings.autoconfig.boardSheet.allBrands')}</span>
            <span className="bps-menu__meta">{t('settings.autoconfig.boardSheet.clearBrand')}</span>
          </span>
          <kbd className="bps-kbd bps-menu__go" aria-hidden="true">
            <CornerDownLeft size={11} />
          </kbd>
        </div>
      );
    }
    const pickedHere = v.boards.reduce((n, b) => n + (picker.picked.has(b.slug) ? 1 : 0), 0);
    const showPeek = isActive && peek.length > 0;
    const more = v.boards.length - peek.length;
    return (
      <div
        key={option.key}
        className={`bps-menu__row${isActive ? ' is-active' : ''}${current?.id === v.id ? ' is-current' : ''}`}
        {...common}
      >
        <VendorLogo name={v.name} src={v.logo} />
        <span className="bps-menu__main">
          <span className="bps-menu__name">
            <HighlightText text={v.name} needle={needle} />
          </span>
          <span className="bps-menu__meta">
            {t('home.boardCount', { count: v.boards.length })}
            {pickedHere > 0 && (
              <>
                <i className="bps-menu__dot" aria-hidden="true" />
                <span className="bps-menu__picked">
                  <Check size={10} strokeWidth={3.6} aria-hidden="true" />
                  {t('settings.autoconfig.editor.selectedCount', { count: pickedHere })}
                </span>
              </>
            )}
          </span>
        </span>
        {v.tier && !showPeek && <span className={`mfr-tier is-${v.tier}`}>{v.tier}</span>}
        {showPeek && (
          <span className="bps-menu__peek" aria-hidden="true">
            {peek.map((b, i) => (
              <span key={b.slug} className={`bps-peek${picker.picked.has(b.slug) ? ' is-on' : ''}`} style={{ '--i': i } as React.CSSProperties}>
                <SheetPhoto src={peekPhotos[b.slug]} className="bps-peek__img" />
              </span>
            ))}
            {more > 0 && (
              <span className="bps-peek is-more" style={{ '--i': peek.length } as React.CSSProperties}>
                +{more}
              </span>
            )}
          </span>
        )}
        {current?.id === v.id && (
          <span className="bps-menu__current" title={t('settings.autoconfig.boardSheet.showingNow')}>
            <Check size={10} strokeWidth={3.6} aria-hidden="true" />
          </span>
        )}
        <kbd className="bps-kbd bps-menu__go" aria-hidden="true">
          <CornerDownLeft size={11} />
        </kbd>
      </div>
    );
  };

  return (
    <div
      ref={rootRef}
      className={`bps-menu${isExiting ? ' is-exiting' : ''}`}
      role="dialog"
      aria-label={t('settings.autoconfig.boardSheet.chooseBrand')}
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button, input, [role="option"]')) inputRef.current?.focus();
      }}
    >
      <div className="bps-menu__field">
        <Search size={18} aria-hidden="true" />
        <input
          ref={inputRef}
          type="text"
          value={filter}
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={active ? optionId(active.key) : undefined}
          aria-label={t('settings.autoconfig.boardSheet.filterBrands')}
          placeholder={t('settings.autoconfig.boardSheet.searchBrands', { count: vendors.length })}
          spellCheck={false}
          autoComplete="off"
          onChange={(e) => {
            setFilter(e.target.value);
            keys.setActive(0);
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape' && filter) {
              e.preventDefault();
              e.stopPropagation();
              setFilter('');
              keys.setActive(0);
              return;
            }
            keys.onKeyDown(e);
          }}
        />
        {filter && (
          <button
            type="button"
            className="bps-search__clear"
            tabIndex={-1}
            aria-label={t('modal.clearSearch')}
            onClick={() => {
              setFilter('');
              keys.setActive(0);
              inputRef.current?.focus();
            }}
          >
            <X size={11} strokeWidth={3} aria-hidden="true" />
          </button>
        )}
        <kbd className="bps-kbd" aria-hidden="true">
          {t('settings.autoconfig.boardSheet.escKey')}
        </kbd>
      </div>

      <div ref={scrollRef} className="bps-menu__scroll">
        <div id={listId} role="listbox" aria-label={t('settings.autoconfig.boardSheet.chooseBrand')}>
          {sections.map((section, sectionIndex) => (
            <div
              key={section.id}
              className={`bps-menu__section${section.label ? '' : ' bps-menu__section--bare'}`}
              role="group"
              aria-label={section.label || undefined}
            >
              {section.label && (
                <div className="bps-menu__head" aria-hidden="true">
                  <span>{section.label}</span>
                  <small>{section.note}</small>
                </div>
              )}
              {section.options.map((option, i) => row(option, starts[sectionIndex] + i))}
            </div>
          ))}
        </div>
        {sections.length === 0 && (
          <div className="bps-menu__empty">
            <span className="bps-menu__empty-icon" aria-hidden="true">
              <SearchX size={20} />
            </span>
            <span className="bps-menu__empty-title">
              {t('settings.autoconfig.boardSheet.noBrandMatchQuery', { query: filter.trim() })}
            </span>
            <span className="bps-menu__empty-hint">{t('settings.autoconfig.boardSheet.noBrandMatchHint')}</span>
            <button type="button" className="pe-pill__btn is-secondary" onClick={() => onSearchBoards(filter.trim())}>
              {t('settings.autoconfig.boardSheet.searchBoardsFor', { query: filter.trim() })}
            </button>
          </div>
        )}
      </div>

      <div className="bps-menu__foot">
        <span className="bps-menu__hint">
          <kbd className="bps-kbd">
            <ArrowUp size={10} />
          </kbd>
          <kbd className="bps-kbd">
            <ArrowDown size={10} />
          </kbd>
          {t('settings.autoconfig.boardSheet.hintMove')}
        </span>
        <span className="bps-menu__hint">
          <kbd className="bps-kbd">
            <CornerDownLeft size={10} />
          </kbd>
          {t('settings.autoconfig.boardSheet.hintOpen')}
        </span>
        <span className="bps-menu__count">
          {t('settings.autoconfig.boardSheet.menuCount', { brands: vendors.length, partners: partners.length })}
        </span>
      </div>
    </div>
  );
}
