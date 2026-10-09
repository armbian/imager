// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { UI } from '../config';

const { COL_MIN, GAP: GRID_GAP, MIN_ROWS } = UI.GRID;

export interface PagedGrid<T> {
  page: number;
  setPage: (page: number) => void;
  pageCount: number;
  safePage: number;
  pagedItems: T[];
  pageSize: number;
  /** Callback ref for the scrollable grid element; drives the dynamic page size. */
  measureGrid: (node: HTMLElement | null) => void;
}

/** A grid laid out by CSS `repeat(auto-fill, minmax(colMin, 1fr))` with `gap`; rows are whole cards only. */
export interface GridFit {
  colMin: number;
  gap: number;
  minRows: number;
}

const HOME_GRID: GridFit = { colMin: COL_MIN, gap: GRID_GAP, minRows: MIN_ROWS };

/** Paginates `items` (already sorted/filtered) into pages sized to fit the measured grid viewport (cols × rows),
 *  capped at `maxPerPage` to avoid lag. `cardRow` = card row height+gap px; `resetKey` change resets to page 1. */
export function usePagedGrid<T>(
  items: T[],
  cardRow: number,
  resetKey: unknown,
  maxPerPage: number = UI.GRID.MAX_PER_PAGE,
  fit: GridFit = HOME_GRID
): PagedGrid<T> {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState<number>(UI.GRID.INITIAL_PAGE_SIZE);
  const observerRef = useRef<ResizeObserver | null>(null);

  // Measure the grid viewport and derive how many cards (cols × rows) fit; callback ref (re)attaches the
  // observer on mount. Viewport is flex-sized, so it doesn't depend on rendered content, and setting the
  // card fill height on it never resizes it.
  const measureGrid = useCallback(
    (node: HTMLElement | null) => {
      observerRef.current?.disconnect();
      observerRef.current = null;
      if (!node) return;
      const recompute = () => {
        const w = node.clientWidth;
        const h = node.clientHeight;
        if (w === 0 || h === 0) return;
        const pad = getComputedStyle(node);
        const innerW = w - parseFloat(pad.paddingLeft) - parseFloat(pad.paddingRight);
        const innerH = h - parseFloat(pad.paddingTop) - parseFloat(pad.paddingBottom);
        const cols = Math.max(1, Math.floor((innerW + fit.gap) / (fit.colMin + fit.gap)));
        // A grid that declares --grid-card-min/max lets its cards stretch so whole rows fill the height exactly
        const cardMin = parseFloat(pad.getPropertyValue('--grid-card-min'));
        const cardMax = parseFloat(pad.getPropertyValue('--grid-card-max'));
        const stretch = cardMin > 0 && cardMax >= cardMin;
        const rowsFit = Math.floor((innerH + fit.gap) / (stretch ? cardMin + fit.gap : cardRow));
        // Cap by whole rows so the page is always full rows of `cols`, never a
        // partial last row (e.g. 3+3+2), which is what made the last card vanish.
        const maxRows = Math.max(1, Math.floor(maxPerPage / cols));
        const rows = Math.max(1, Math.min(Math.max(rowsFit, fit.minRows), maxRows));
        if (stretch) {
          const fill = Math.floor((innerH - (rows - 1) * fit.gap) / rows);
          node.style.setProperty('--grid-card-fill', `${Math.min(cardMax, Math.max(cardMin, fill))}px`);
        }
        const size = cols * rows;
        setPageSize((prev) => (prev === size ? prev : size));
      };
      recompute();
      observerRef.current = new ResizeObserver(recompute);
      observerRef.current.observe(node);
    },
    [cardRow, maxPerPage, fit]
  );

  useEffect(() => () => observerRef.current?.disconnect(), []);

  // Reset to the first page whenever the result set changes (e.g. a new search).
  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- reset paging on a new result set
    setPage(1);
  }, [resetKey]);

  const pageCount = Math.max(1, Math.ceil(items.length / pageSize));
  const safePage = Math.min(page, pageCount);
  const pagedItems = useMemo(
    () => items.slice((safePage - 1) * pageSize, safePage * pageSize),
    [items, safePage, pageSize]
  );

  return { page, setPage, pageCount, safePage, pagedItems, pageSize, measureGrid };
}
