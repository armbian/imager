// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useLayoutEffect, useRef, type CSSProperties } from 'react';

interface GridPagerProps {
  pageCount: number;
  page: number;
  onChange: (page: number) => void;
  ariaLabel?: string;
  compact?: boolean;
}

/** Windowed page list with ellipses, e.g. [1,'gap',4,5,6,'gap',12]; always keeps
 * first/last page plus a one-page window around current. */
function buildPageList(current: number, count: number): Array<number | 'gap'> {
  const out: Array<number | 'gap'> = [];
  for (let p = 1; p <= count; p++) {
    if (p === 1 || p === count || (p >= current - 1 && p <= current + 1)) {
      out.push(p);
    } else if (out[out.length - 1] !== 'gap') {
      out.push('gap');
    }
  }
  return out;
}

/** Writes the compact pill's geometry (resting capsule vs full row, the disc's trip to its slot) as the CSS variables
 * its morph reads. Layout offsets ignore transforms, so this is safe mid-animation. */
function measureMorph(nav: HTMLElement, row: HTMLElement, mini: HTMLElement, slot: HTMLElement) {
  const active = row.querySelector<HTMLElement>('[aria-current="page"]');
  if (!active) return;
  const disc = row.offsetLeft + active.offsetLeft + active.offsetWidth / 2;
  const shrink = mini.offsetHeight / nav.offsetHeight;
  const vars: Array<[string, string]> = [
    ['--pager-shrink', String(shrink)],
    ['--pager-fold', `${(nav.offsetWidth * shrink - mini.offsetWidth) / 2}px`],
    ['--pager-inset-x', `${(nav.offsetWidth - mini.offsetWidth) / 2}px`],
    ['--pager-inset-y', `${(nav.offsetHeight - mini.offsetHeight) / 2}px`],
    ['--pager-lead', `${mini.offsetLeft + slot.offsetLeft + slot.offsetWidth / 2 - disc}px`],
    ['--pager-disc', String(slot.offsetHeight / active.offsetHeight)],
  ];
  // Folded, a new value would animate the resting pill; open, nothing on screen reads these values
  const folded = !nav.matches(':hover, :focus-within');
  if (folded) nav.dataset.measuring = '';
  vars.forEach(([name, value]) => nav.style.setProperty(name, value));
  for (const item of Array.from(row.children) as HTMLElement[]) {
    item.style.setProperty('--pager-off', `${row.offsetLeft + item.offsetLeft + item.offsetWidth / 2 - disc}px`);
  }
  if (folded) {
    void nav.offsetWidth;
    delete nav.dataset.measuring;
  }
}

/** Floating frosted pill of numbered page buttons; renders nothing for a single page. With `compact`, for a
 * pill that floats over content, it rests as a small "page / count" capsule and stretches open on hover or focus. */
export function GridPager({ pageCount, page, onChange, ariaLabel, compact = false }: GridPagerProps) {
  const navRef = useRef<HTMLElement>(null);
  const rowRef = useRef<HTMLSpanElement>(null);
  const miniRef = useRef<HTMLSpanElement>(null);
  const slotRef = useRef<HTMLElement>(null);

  useLayoutEffect(() => {
    const nav = navRef.current;
    const row = rowRef.current;
    const mini = miniRef.current;
    const slot = slotRef.current;
    if (!compact || !nav || !row || !mini || !slot) return;
    const measure = () => measureMorph(nav, row, mini, slot);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(row);
    observer.observe(mini);
    return () => observer.disconnect();
  }, [compact, page, pageCount]);

  if (pageCount <= 1) return null;
  const list = buildPageList(page, pageCount);
  const activeIndex = list.indexOf(page);
  const rank = (i: number) => (compact ? ({ '--pager-rank': Math.abs(i - activeIndex) } as CSSProperties) : undefined);
  const pages = (
    <span ref={rowRef} className="mfr-pager__row">
      {list.map((item, i) =>
        item === 'gap' ? (
          <span key={`gap-${i}`} className="mfr-pager__gap" style={rank(i)} aria-hidden="true">
            …
          </span>
        ) : (
          <button
            key={item}
            type="button"
            className={`mfr-pager__page${item === page ? ' is-active' : ''}`}
            style={rank(i)}
            onClick={() => onChange(item)}
            aria-current={item === page ? 'page' : undefined}
          >
            <span className="mfr-pager__num">{item}</span>
          </button>
        )
      )}
    </span>
  );

  return (
    <nav ref={navRef} className={`mfr-pager${compact ? ' mfr-pager--compact' : ''}`} aria-label={ariaLabel}>
      {compact && (
        <>
          <span className="mfr-pager__door" aria-hidden="true">
            <span className="mfr-pager__cap" />
          </span>
          <span className="mfr-pager__door" aria-hidden="true">
            <span className="mfr-pager__cap" />
          </span>
          <span className="mfr-pager__glass" aria-hidden="true" />
          <span ref={miniRef} className="mfr-pager__mini" aria-hidden="true">
            <b ref={slotRef}>{page}</b>/ {pageCount}
          </span>
        </>
      )}
      {pages}
    </nav>
  );
}
