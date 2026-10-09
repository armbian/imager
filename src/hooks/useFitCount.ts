// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useLayoutEffect, useState, type RefObject } from 'react';

const right = (el: HTMLElement) => el.offsetLeft + el.offsetWidth;

/** Items of a one-line row that fit whole, measured on `measureRef`, an unclipped hidden copy: a lead, every item, the pill */
export function useFitCount(
  rowRef: RefObject<HTMLElement | null>,
  measureRef: RefObject<HTMLElement | null>,
  /** More are left out beyond the measured items, so the pill shows even when they all fit */
  overflowBeyond: boolean,
  /** Changes whenever the items or their text change */
  contentKey: string
): number {
  const [fit, setFit] = useState(0);

  useLayoutEffect(() => {
    const row = rowRef.current;
    const copy = measureRef.current;
    if (!row || !copy) return;
    const compute = () => {
      const kids = [...copy.children] as HTMLElement[];
      if (kids.length < 2) return;
      const lead = kids[0];
      const pill = kids[kids.length - 1];
      const items = kids.slice(1, -1);
      const gap = parseFloat(getComputedStyle(copy).columnGap) || 0;
      const avail = row.clientWidth;
      const fits = (count: number) => {
        const end = count ? right(items[count - 1]) : right(lead);
        const needsPill = overflowBeyond || count < items.length;
        return (needsPill ? end + gap + pill.offsetWidth : end) <= avail;
      };
      let count = items.length;
      while (count > 0 && !fits(count)) count--;
      setFit(count);
    };
    compute();
    const observer = new ResizeObserver(compute);
    observer.observe(row);
    return () => observer.disconnect();
  }, [rowRef, measureRef, overflowBeyond, contentKey]);

  return fit;
}
