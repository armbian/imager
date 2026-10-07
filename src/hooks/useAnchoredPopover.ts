// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type RefObject } from 'react';
import { UI } from '../config';

interface PopoverOptions {
  width: number | null;
  prefer: 'up' | 'down';
  align: 'start' | 'end';
  gap: number;
  onClose: () => void;
}

/** Fixed popover beside its trigger that flips, follows scroll and closes on an outside press. */
export function useAnchoredPopover<T extends HTMLElement>(
  anchorRef: RefObject<HTMLElement | null>,
  open: boolean,
  { width, prefer, align, gap, onClose }: PopoverOptions
): { popRef: RefObject<T | null>; style: CSSProperties; placement: 'up' | 'down' } {
  const popRef = useRef<T | null>(null);
  // Transparent, not hidden, until placed: an autofocused field inside must still take focus.
  const [style, setStyle] = useState<CSSProperties>({ opacity: 0 });
  const [placement, setPlacement] = useState<'up' | 'down'>(prefer);
  const onCloseRef = useRef(onClose);
  useLayoutEffect(() => {
    onCloseRef.current = onClose;
  }, [onClose]);

  const place = useCallback(() => {
    const anchor = anchorRef.current;
    const pop = popRef.current;
    if (!anchor || !pop) return;
    const edge = UI.POPOVER_EDGE;
    const a = anchor.getBoundingClientRect();
    const w = width ?? a.width;
    const h = pop.offsetHeight;
    const roomUp = a.top - gap - edge;
    const roomDown = window.innerHeight - a.bottom - gap - edge;
    const up = prefer === 'up' ? roomUp >= h || roomUp > roomDown : !(roomDown >= h || roomDown > roomUp);
    const top = up ? Math.max(edge, a.top - gap - h) : Math.min(a.bottom + gap, window.innerHeight - edge - h);
    const rawLeft = align === 'end' ? a.right - w : a.left;
    const left = Math.min(Math.max(edge, rawLeft), window.innerWidth - edge - w);
    setPlacement(up ? 'up' : 'down');
    setStyle({ top, left, width: w });
  }, [anchorRef, width, prefer, align, gap]);

  useLayoutEffect(() => {
    if (!open) return;
    // eslint-disable-next-line react-hooks/set-state-in-effect -- placed from its own measured size once mounted
    place();
  }, [open, place]);

  useEffect(() => {
    if (!open) return;
    const onDown = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (!target) return;
      if (popRef.current?.contains(target) || anchorRef.current?.contains(target)) return;
      onCloseRef.current();
    };
    window.addEventListener('pointerdown', onDown, true);
    window.addEventListener('resize', place);
    window.addEventListener('scroll', place, true);
    return () => {
      window.removeEventListener('pointerdown', onDown, true);
      window.removeEventListener('resize', place);
      window.removeEventListener('scroll', place, true);
    };
  }, [open, place, anchorRef]);

  return { popRef, style, placement };
}
