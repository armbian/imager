// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useId, useRef, useState, type KeyboardEvent, type RefObject } from 'react';
import { createPortal } from 'react-dom';
import { useTranslation } from 'react-i18next';
import { Check, ChevronLeft, X } from 'lucide-react';
import { UI } from '../../config';
import { useBoardPicker } from '../../hooks/useBoardPicker';
import { useDialogFocus } from '../../hooks/useDialogFocus';
import { useModalExitAnimation } from '../../hooks/useModalExitAnimation';
import type { ProfileBoard } from '../../types';
import { focusAfterInput } from '../../utils';
import { BoardLedger, BoardSearch, BrowseBar, BrowseGrid } from './BoardPicker';
import { BoardReview } from './BoardReview';
import { BrandMenu } from './BrandMenu';

interface BoardPickerSheetProps {
  selected: ProfileBoard[];
  onDone: (boards: ProfileBoard[]) => void;
  onCancel: () => void;
  /** The profile being edited, left out of the menu's "Recent" brands */
  profileId?: string;
  returnFocusRef?: RefObject<HTMLElement | null>;
}

interface Undoable {
  id: number;
  message: string;
  undo: () => void;
}

/** Choose boards: a paged catalog with a brand menu and search, a ledger of the picks and a review of them. */
export function BoardPickerSheet({ selected, onDone, onCancel, profileId, returnFocusRef }: BoardPickerSheetProps) {
  const { t } = useTranslation();
  const titleId = useId();
  const [draft, setDraft] = useState<ProfileBoard[]>(selected);
  const picker = useBoardPicker(draft, setDraft, profileId);
  const [menu, setMenu] = useState<{ filter: string } | null>(null);
  const [undoable, setUndoable] = useState<Undoable | null>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const review = picker.view === 'review';
  // Only a flip between browse and review slides the faces; the first browse face arrives with the sheet
  const [shownView, setShownView] = useState(picker.view);
  const [flipped, setFlipped] = useState(false);
  if (shownView !== picker.view) {
    setShownView(picker.view);
    setFlipped(true);
  }

  const result = useRef<ProfileBoard[] | null>(null);
  const { isExiting, handleClose } = useModalExitAnimation({
    onClose: () => (result.current ? onDone(result.current) : onCancel()),
  });
  const cancel = () => {
    result.current = null;
    handleClose();
  };
  const finish = () => {
    if (!draft.length) return;
    result.current = draft;
    handleClose();
  };
  const dialogRef = useDialogFocus<HTMLDivElement>({ onEscape: () => (review ? picker.setView('browse') : cancel()), returnFocusRef });

  useEffect(() => {
    if (!undoable) return;
    const timer = setTimeout(() => setUndoable(null), UI.BOARD_SHEET.UNDO_MS);
    return () => clearTimeout(timer);
  }, [undoable]);

  const pointerRef = useRef(false);
  const openMenu = (filter: string) => setMenu((current) => current ?? { filter });
  // The menu's field had focus; hand it back to the trigger unless the click that closed it focused something else
  const closeMenu = () => {
    setMenu(null);
    requestAnimationFrame(() => {
      if (document.activeElement === document.body && searchRef.current) focusAfterInput(searchRef.current, pointerRef.current);
    });
  };
  const searchBoards = (query: string) => {
    setMenu(null);
    picker.setQuery(query);
    searchRef.current?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const typing = (event.target as HTMLElement).closest('input');
    if (typing || menu || event.defaultPrevented) return;
    if (event.key === UI.BOARD_SHEET.SEARCH_KEY && !review) {
      event.preventDefault();
      searchRef.current?.focus();
      return;
    }
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    const step = event.key === 'ArrowRight' ? 1 : -1;
    const [current, count, go] = review
      ? [picker.reviewPage, picker.reviewPageCount, picker.setReviewPage]
      : [picker.page, picker.pageCount, picker.setPage];
    const next = current + step;
    if (next < 1 || next > count) return;
    event.preventDefault();
    go(next);
  };

  return createPortal(
    <div className={`modal-overlay ${isExiting ? 'modal-exiting' : 'modal-entering'}`} onClick={cancel}>
      <div
        ref={dialogRef}
        className={`bps${review ? ' is-review' : ''}${flipped ? ' is-flipped' : ''}${isExiting ? ' is-exiting' : ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKeyDown}
        onPointerDownCapture={() => (pointerRef.current = true)}
        onKeyDownCapture={() => (pointerRef.current = false)}
      >
        <header className="bps__head">
          {review ? (
            <div key="review" className="bps__face is-review">
              <button
                type="button"
                className="bps__back"
                aria-label={t('settings.autoconfig.boardSheet.backToAll')}
                autoFocus
                onClick={() => picker.setView('browse')}
              >
                <ChevronLeft size={16} strokeWidth={2.4} aria-hidden="true" />
              </button>
              <span className="bps__titles">
                <h2 id={titleId} className="bps__title">
                  {t('settings.autoconfig.boardSheet.reviewTitle')}
                </h2>
                <span className="bps__sub">{t('settings.autoconfig.boardSheet.reviewHint')}</span>
              </span>
              <button type="button" className="modal-close" onClick={cancel} aria-label={t('common.close')}>
                <X size={20} aria-hidden="true" />
              </button>
            </div>
          ) : (
            <div key="browse" className="bps__face">
              <h2 id={titleId} className="bps__title">
                {t('settings.autoconfig.editor.chooseBoards')}
              </h2>
              <BoardSearch picker={picker} inputRef={searchRef} />
              <button type="button" className="modal-close" onClick={cancel} aria-label={t('common.close')}>
                <X size={20} aria-hidden="true" />
              </button>
            </div>
          )}
        </header>

        <div className="bps__clip">
          <div key={picker.view} className="bps__body">
            {review ? (
              <BoardReview
                picker={picker}
                onUndoable={(message, undo) => setUndoable({ id: Date.now(), message, undo })}
              />
            ) : (
              <>
                <BrowseBar
                  picker={picker}
                  onMoreBrands={() => openMenu(picker.query)}
                  menu={
                    menu && (
                      <BrandMenu
                        picker={picker}
                        initialFilter={menu.filter}
                        onClose={closeMenu}
                        onSearchBoards={searchBoards}
                      />
                    )
                  }
                />
                <BrowseGrid picker={picker} onClearSearch={() => searchBoards('')} />
              </>
            )}
          </div>

          <footer className="bps__foot">
            <BoardLedger picker={picker} />
            <span className="bps__actions">
              <button type="button" className="pe-pill__btn is-secondary" onClick={cancel}>
                {t('common.cancel')}
              </button>
              <button
                type="button"
                className="pe-pill__btn is-primary"
                aria-disabled={draft.length === 0}
                title={draft.length === 0 ? t('settings.autoconfig.editor.saveNeedsBoard') : undefined}
                onClick={finish}
              >
                {t('settings.autoconfig.editor.done')}
              </button>
            </span>
          </footer>

          {undoable && (
            <div key={undoable.id} className="bps-undo" role="status">
              <span className="bps-undo__icon" aria-hidden="true">
                <Check size={12} strokeWidth={3} />
              </span>
              <span className="bps-undo__text">{undoable.message}</span>
              <button
                type="button"
                className="bps-undo__btn"
                onClick={() => {
                  undoable.undo();
                  setUndoable(null);
                }}
              >
                {t('settings.autoconfig.boardSheet.undo')}
              </button>
            </div>
          )}
        </div>
      </div>
    </div>,
    document.body
  );
}
