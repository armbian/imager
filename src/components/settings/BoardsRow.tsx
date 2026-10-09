// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useMemo, useRef, useState, type MouseEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, X } from 'lucide-react';
import { UI } from '../../config/constants';
import { useAsyncData } from '../../hooks/useAsyncData';
import { useBoardPhotos } from '../../hooks/useBoardPhotos';
import { useLeavingItem } from '../../hooks/useLeavingItem';
import { getBoards } from '../../hooks/useTauri';
import type { BoardInfo, ProfileBoard } from '../../types';
import { staggerDelay, stripVendorPrefix } from '../../utils';
import { BoardImage, GridPager } from '../shared';
import { BoardsEmpty } from './BoardsEmpty';

interface BoardsRowProps {
  boards: ProfileBoard[];
  onEdit: (event: MouseEvent<HTMLElement>) => void;
  onRemove: (slug: string) => void;
  /** At least one board is needed: the empty panel gets an amber hairline */
  required?: boolean;
  /** Fixed columns; omit to fit as many as the width allows */
  slots?: number;
}

const { MIN_CARD_WIDTH, GAP, MIN_SLOTS, DEFAULT_SLOTS } = UI.BOARDS_ROW;

// The name clamps to two lines; it gets a tooltip only when that clamp actually hides part of it
function titleIfClamped(event: MouseEvent<HTMLElement>, name: string) {
  const el = event.currentTarget;
  el.title = el.scrollHeight > el.clientHeight + 1 ? name : '';
}

/** The chosen boards as mini cards, a page at a time, or an empty panel with Add board while none is picked */
export function BoardsRow({ boards, onEdit, onRemove, required = false, slots: fixedSlots }: BoardsRowProps) {
  const { t } = useTranslation();
  const { leaving, leave } = useLeavingItem();
  const listRef = useRef<HTMLUListElement>(null);
  const [measured, setMeasured] = useState<number>(DEFAULT_SLOTS);
  const slots = fixedSlots ?? measured;
  const [page, setPage] = useState(1);
  const empty = boards.length === 0;

  useEffect(() => {
    const el = listRef.current;
    if (!el || fixedSlots) return;
    const measure = () => {
      if (el.clientWidth < MIN_CARD_WIDTH) return;
      setMeasured(Math.max(MIN_SLOTS, Math.floor((el.clientWidth + GAP) / (MIN_CARD_WIDTH + GAP))));
    };
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [fixedSlots, empty]);

  const perPage = slots;
  const pageCount = Math.max(1, Math.ceil(boards.length / perPage));
  const safePage = Math.min(page, pageCount);
  const shown = boards.slice((safePage - 1) * perPage, safePage * perPage);
  const photos = useBoardPhotos(shown.map((b) => b.slug));

  // The card fades out first, then the board leaves the list; the page clamps through safePage
  const remove = (slug: string) => leave(slug, () => onRemove(slug));
  // Profiles store slug and name only; the vendor line comes from the cached catalog and is left out offline
  const { data: catalog } = useAsyncData<BoardInfo[]>(() => getBoards().catch(() => []), []);
  const vendorOf = useMemo(() => new Map((catalog ?? []).map((b) => [b.slug, b.vendor_name || ''])), [catalog]);

  const addLabel = t('settings.autoconfig.editor.addBoard');

  return (
    <div className="boards-row">
      {empty ? (
        <BoardsEmpty
          warn={required}
          title={t('settings.autoconfig.editor.boardsEmptyTitle')}
          text={t('settings.autoconfig.editor.boardsEmptyText')}
          action={
            <>
              <button type="button" className="btn btn-primary btn-pill boards-row__add" onClick={onEdit}>
                <Plus size={16} aria-hidden="true" />
                {addLabel}
              </button>
              <span className="boards-empty__hint">{t('settings.autoconfig.editor.boardsEmptyHint')}</span>
            </>
          }
        />
      ) : (
        <>
          <ul ref={listRef} className="boards-row__list" style={{ gridTemplateColumns: `repeat(${slots}, minmax(0, 1fr))` }}>
            {shown.map((b, index) => {
              const vendor = vendorOf.get(b.slug) ?? '';
              return (
                <li key={b.slug}>
                  {/* Keyed by slug: a page change mounts new cards that play the home entrance, a removal replays nothing */}
                  <div
                    className={`bps-card boards-row__card mfr-card--enter${leaving === b.slug ? ' is-leaving' : ''}`}
                    style={{ animationDelay: staggerDelay(index) }}
                  >
                    <span className="bps-card__img">
                      {b.slug in photos ? <BoardImage src={photos[b.slug]} alt="" /> : <span className="sk-shim bps-card__shim" />}
                    </span>
                    <span className="bps-card__info">
                      {vendor && <span className="bps-card__vendor">{vendor}</span>}
                      <span className="bps-card__name" onMouseEnter={(e) => titleIfClamped(e, b.name)}>
                        {stripVendorPrefix(b.name, vendor)}
                      </span>
                    </span>
                    <button
                      type="button"
                      className="boards-row__remove"
                      aria-label={t('settings.autoconfig.editor.removeBoard', { name: b.name })}
                      title={t('settings.autoconfig.editor.removeBoard', { name: b.name })}
                      disabled={leaving !== null}
                      onClick={() => remove(b.slug)}
                    >
                      <X size={14} aria-hidden="true" />
                    </button>
                  </div>
                </li>
              );
            })}
          </ul>
          <GridPager compact pageCount={pageCount} page={safePage} onChange={setPage} ariaLabel={t('settings.autoconfig.editor.boardPages')} />
        </>
      )}
    </div>
  );
}
