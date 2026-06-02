import { useState, useEffect, useMemo, useRef, useCallback } from 'react';

/** Default upper bound, sized to avoid loading too many heavy cards (board
 *  photos) at once. Callers with light cards (vendor logos) can raise it. */
const DEFAULT_MAX_PER_PAGE = 40;
// Grid metrics mirrored from CSS (.mfr-grid) for the column/row fit calculation.
const COL_MIN = 220;
const GRID_GAP = 18;
const GRID_PAD = 24;
/** Always show at least this many full rows so a short window isn't left with a
 *  single lonely row (the page scrolls; bottom padding clears the floating pager). */
const MIN_ROWS = 2;

export interface PagedGrid<T> {
  page: number;
  setPage: (page: number) => void;
  pageCount: number;
  safePage: number;
  pagedItems: T[];
  /** Callback ref for the scrollable grid element; drives the dynamic page size. */
  measureGrid: (node: HTMLElement | null) => void;
}

/** Paginates `items` (already sorted/filtered) into pages sized to fit the measured grid viewport (cols × rows),
 *  capped at `maxPerPage` to avoid lag. `cardRow` = card row height+gap px; `resetKey` change resets to page 1. */
export function usePagedGrid<T>(
  items: T[],
  cardRow: number,
  resetKey: unknown,
  maxPerPage: number = DEFAULT_MAX_PER_PAGE
): PagedGrid<T> {
  const [page, setPage] = useState(1);
  const [pageSize, setPageSize] = useState(15);
  const observerRef = useRef<ResizeObserver | null>(null);

  // Measure the grid viewport and derive how many cards (cols × rows) fit; callback ref (re)attaches the
  // observer on mount. Viewport is flex-sized, so it doesn't depend on rendered content.
  const measureGrid = useCallback(
    (node: HTMLElement | null) => {
      observerRef.current?.disconnect();
      observerRef.current = null;
      if (!node) return;
      const recompute = () => {
        const w = node.clientWidth;
        const h = node.clientHeight;
        if (w === 0 || h === 0) return;
        const cols = Math.max(1, Math.floor((w - GRID_PAD * 2 + GRID_GAP) / (COL_MIN + GRID_GAP)));
        // Rows that fill the viewport (a near-fit row is rounded in so the space
        // isn't wasted; the page scrolls slightly and the last row clears the pill).
        const rowsFit = Math.round((h - GRID_PAD) / cardRow);
        // Cap by whole rows so the page is always full rows of `cols` — never a
        // partial last row (e.g. 3+3+2), which is what made the last card vanish.
        const maxRows = Math.max(1, Math.floor(maxPerPage / cols));
        const rows = Math.max(1, Math.min(Math.max(rowsFit, MIN_ROWS), maxRows));
        const size = cols * rows;
        setPageSize((prev) => (prev === size ? prev : size));
      };
      recompute();
      observerRef.current = new ResizeObserver(recompute);
      observerRef.current.observe(node);
    },
    [cardRow, maxPerPage]
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

  return { page, setPage, pageCount, safePage, pagedItems, measureGrid };
}
