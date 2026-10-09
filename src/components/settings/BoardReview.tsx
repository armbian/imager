// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { ChevronDown, CircuitBoard, X } from 'lucide-react';
import { SUPPORT_TIER_LABEL, SUPPORT_TIER_ORDER, UI } from '../../config';
import type { BoardPickerState, PickedBoard, ReviewFilter } from '../../hooks/useBoardPicker';
import { useLeavingItem } from '../../hooks/useLeavingItem';
import { GridPager, PageSwap } from '../shared';
import { SheetCard } from './BoardPicker';
import { VendorLogo } from './VendorLogo';

interface BoardReviewProps {
  picker: BoardPickerState;
  onUndoable: (message: string, undo: () => void) => void;
}

const sameFilter = (a: ReviewFilter, kind: 'tier' | 'vendor', value: string) => !!a && a.kind === kind && a.value === value;

/** Review face: a summary card that filters by tier or brand, and the picked boards with a remove button each. */
export function BoardReview({ picker, onUndoable }: BoardReviewProps) {
  const { t } = useTranslation();
  const { picks, reviewFilter, setReviewFilter, reviewItems, photos, vendorById } = picker;
  const [allVendors, setAllVendors] = useState(false);
  const { leaving, leave } = useLeavingItem();

  const tiers = SUPPORT_TIER_ORDER.map((tier) => [tier, picks.filter((p) => p.board?.support_tier === tier).length] as const).filter(
    ([, n]) => n > 0
  );
  const vendorCounts = new Map<string, number>();
  for (const p of picks) if (p.board?.vendor) vendorCounts.set(p.board.vendor, (vendorCounts.get(p.board.vendor) ?? 0) + 1);
  const vendorRows = [...vendorCounts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const collapse = !allVendors && vendorRows.length > UI.BOARD_SHEET.REVIEW_VENDORS + 1;
  const shownVendors = collapse ? vendorRows.slice(0, UI.BOARD_SHEET.REVIEW_VENDORS) : vendorRows;

  const toggleFilter = (kind: 'tier' | 'vendor', value: string) =>
    setReviewFilter(sameFilter(reviewFilter, kind, value) ? null : { kind, value });

  const remove = (p: PickedBoard) => {
    const before = picks.map(({ slug, name }) => ({ slug, name }));
    leave(p.slug, () => {
      picker.remove(p.slug);
      onUndoable(t('settings.autoconfig.boardSheet.removed', { name: p.name }), () => picker.replace(before));
    });
  };

  const clearAll = () => {
    const before = picks.map(({ slug, name }) => ({ slug, name }));
    picker.replace([]);
    onUndoable(t('settings.autoconfig.boardSheet.cleared', { count: before.length }), () => picker.replace(before));
  };

  return (
    <div className="bps-review">
      <aside className="bps-sum">
        <span className="bps-sum__big">{picks.length}</span>
        <span className="bps-sum__cap">{t('settings.autoconfig.boardSheet.reviewCount', { count: picks.length })}</span>

        {tiers.length > 0 && (
          <>
            <span className="bps-sum__label">{t('settings.autoconfig.boardSheet.byTier')}</span>
            {tiers.map(([tier, n]) => (
              <button
                key={tier}
                type="button"
                className={`bps-sum__row${sameFilter(reviewFilter, 'tier', tier) ? ' is-on' : ''}`}
                aria-pressed={sameFilter(reviewFilter, 'tier', tier)}
                onClick={() => toggleFilter('tier', tier)}
              >
                <span className={`bp-tier is-${tier}`}>{SUPPORT_TIER_LABEL[tier]}</span>
                <b>{n}</b>
              </button>
            ))}
          </>
        )}

        {vendorRows.length > 0 && (
          <>
            <span className="bps-sum__label">
              {t('settings.autoconfig.boardSheet.byVendor')}
              <span className="bps-sum__label-count"> · {vendorRows.length}</span>
            </span>
            <div className="bps-sum__vendors">
              {shownVendors.map(([id, n]) => {
                const v = vendorById.get(id);
                const name = v?.name ?? id;
                return (
                  <button
                    key={id}
                    type="button"
                    className={`bps-sum__row${sameFilter(reviewFilter, 'vendor', id) ? ' is-on' : ''}`}
                    aria-pressed={sameFilter(reviewFilter, 'vendor', id)}
                    onClick={() => toggleFilter('vendor', id)}
                  >
                    <VendorLogo name={name} src={v?.logo ?? null} size="xs" />
                    <span className="bps-sum__name">{name}</span>
                    <b>{n}</b>
                  </button>
                );
              })}
              {collapse && (
                <button type="button" className="bps-sum__row is-more" onClick={() => setAllVendors(true)}>
                  <span className="bps-sum__more" aria-hidden="true">
                    <ChevronDown size={10} strokeWidth={3} />
                  </span>
                  <span className="bps-sum__name">
                    {t('settings.autoconfig.boardSheet.moreVendors', { count: vendorRows.length - shownVendors.length })}
                  </span>
                </button>
              )}
            </div>
          </>
        )}

        <span className="bps-sum__foot">
          <button type="button" className="bps-sum__action" disabled={picks.length === 0} onClick={clearAll}>
            {t('settings.autoconfig.boardSheet.clearAll')}
          </button>
          {reviewFilter && (
            <button type="button" className="bps-sum__action is-accent" onClick={() => setReviewFilter(null)}>
              {t('settings.autoconfig.boardSheet.showAll')}
            </button>
          )}
        </span>
      </aside>

      <div className="bps-review__main">
        {picks.length === 0 ? (
          <div className="bps-review__empty device-empty">
            <span className="device-empty__icon" aria-hidden="true">
              <CircuitBoard size={UI.ICON_SIZE.EMPTY_STATE} />
            </span>
            <p className="device-empty__title">{t('settings.autoconfig.boardSheet.reviewEmptyTitle')}</p>
            <p className="device-empty__hint">{t('settings.autoconfig.boardSheet.reviewEmptyHint')}</p>
            <button type="button" className="pe-pill__btn is-primary" onClick={() => picker.setView('browse')}>
              {t('settings.autoconfig.boardSheet.browseBoards')}
            </button>
          </div>
        ) : (
          <div className="bps-stage">
            <PageSwap
              swapKey={`${reviewItems.map((p) => p.slug).join('\x1f')}`}
              motion={picker.motion}
              className="bps__grid bps__grid--review"
              role="list"
              aria-label={t('settings.autoconfig.boardSheet.reviewTitle')}
            >
              {reviewItems.map((p) => (
                <SheetCard
                  key={p.slug}
                  slug={p.slug}
                  name={p.name}
                  board={p.board}
                  photo={photos[p.slug]}
                  className={leaving === p.slug ? 'is-leaving' : ''}
                  action={
                    <button
                      type="button"
                      className="bps-card__remove"
                      aria-label={t('settings.autoconfig.editor.removeBoard', { name: p.name })}
                      title={t('settings.autoconfig.editor.removeBoard', { name: p.name })}
                      disabled={leaving !== null}
                      onClick={() => remove(p)}
                    >
                      <X size={13} strokeWidth={2.8} aria-hidden="true" />
                    </button>
                  }
                />
              ))}
            </PageSwap>
            <GridPager
              compact
              pageCount={picker.reviewPageCount}
              page={picker.reviewPage}
              onChange={picker.setReviewPage}
              ariaLabel={t('settings.autoconfig.boardSheet.selectionPages')}
            />
          </div>
        )}
      </div>
    </div>
  );
}
