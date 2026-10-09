// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useLayoutEffect, useRef, type HTMLAttributes, type ReactNode } from 'react';
import { UI } from '../../config';
import { useMotion } from '../../contexts/MotionContext';

interface PageSwapProps extends HTMLAttributes<HTMLDivElement> {
  /** Identifies the content on screen; a new key swaps the page */
  swapKey: string;
  /** Cause of the change: +1 forward, -1 back, 0 a filter (fade), null no animation (typing) */
  motion: number | null;
  className: string;
  children: ReactNode;
}

/** The outgoing page slides out as an inert copy while the new one slides in; the parent keeps the stage height fixed */
export function PageSwap({ swapKey, motion, className, children, ...rest }: PageSwapProps) {
  const { reduced } = useMotion();
  const gridRef = useRef<HTMLDivElement | null>(null);
  const layerRef = useRef<HTMLDivElement>(null);
  const snapshotRef = useRef<HTMLElement | null>(null);
  const keyRef = useRef(swapKey);
  const timerRef = useRef<number | undefined>(undefined);

  // The keyed grid unmounts on every swap; a copy taken as it leaves is the outgoing layer
  const attach = useCallback((node: HTMLDivElement | null) => {
    gridRef.current = node;
    if (!node) return;
    return () => {
      snapshotRef.current = node.cloneNode(true) as HTMLElement;
    };
  }, []);

  useLayoutEffect(() => {
    const snapshot = snapshotRef.current;
    snapshotRef.current = null;
    if (keyRef.current === swapKey) return;
    keyRef.current = swapKey;
    const layer = layerRef.current;
    const grid = gridRef.current;
    layer?.replaceChildren();
    if (reduced || motion === null || !grid) return;
    if (motion === 0) {
      grid.classList.add('page-swap--fade');
      return;
    }
    const d = Math.sign(motion);
    const stage = grid.parentElement;
    grid.style.setProperty('--d', String(d));
    grid.classList.add('page-swap--in');
    stage?.classList.add('is-swapping');
    grid.addEventListener('animationend', (event) => event.target === grid && stage?.classList.remove('is-swapping'), {
      once: true,
    });
    if (!snapshot || !layer) return;
    snapshot.className = `${className} page-swap__out`;
    snapshot.style.setProperty('--d', String(d));
    snapshot.inert = true;
    layer.append(snapshot);
    const drop = () => snapshot.remove();
    snapshot.addEventListener('animationend', (event) => event.target === snapshot && drop());
    window.clearTimeout(timerRef.current);
    timerRef.current = window.setTimeout(drop, UI.PAGE_SWAP.OUT_FALLBACK_MS);
  }, [swapKey, motion, reduced, className]);

  useEffect(() => () => window.clearTimeout(timerRef.current), []);

  return (
    <div className="page-swap">
      <div key={swapKey} ref={attach} className={className} {...rest}>
        {children}
      </div>
      <div ref={layerRef} className="page-swap__layer" aria-hidden="true" />
    </div>
  );
}
