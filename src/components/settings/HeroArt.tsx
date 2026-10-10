// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState, type CSSProperties } from 'react';
import { SETTINGS } from '../../config';
import { HERO_ART, HERO_SETTLE_MS } from '../../config/heroArt';
import { useTheme } from '../../contexts/ThemeContext';
import type { HeroId } from '../../types';
import { changedEntries, dataAttrs } from '../../utils';

interface HeroArtProps {
  hero: HeroId;
  playing: boolean;
  state?: Record<string, string>;
}

const pct = (value: number, total: number) => `${(value / total) * 100}%`;

/** state keys become data-* attributes; a change after settling also sets data-<key>-to so one-shot reactions replay */
export function HeroArt({ hero, playing, state }: HeroArtProps) {
  const { canvas, layers, settleMs = HERO_SETTLE_MS } = HERO_ART[hero];
  const dark = useTheme().resolvedTheme === SETTINGS.THEME_MODES.DARK;
  const [settled, setSettled] = useState(false);
  const [shownState, setShownState] = useState(state);
  const [events, setEvents] = useState<Record<string, string>>({});

  const changed = changedEntries(shownState ?? {}, state ?? {});
  if (Object.keys(changed).length > 0) {
    setShownState(state);
    if (settled) setEvents(changed);
  }

  useEffect(() => {
    if (!playing) return;
    const id = window.setTimeout(() => setSettled(true), settleMs);
    return () => window.clearTimeout(id);
  }, [playing, settleMs]);

  return (
    <div
      className={`hero-art hero-art--${hero}`}
      data-playing={playing}
      data-settled={settled || undefined}
      aria-hidden="true"
      {...dataAttrs(state ?? {})}
      {...dataAttrs(events, '-to')}
    >
      <div className="hero-art__stack">
        {layers.map((layer, i) => (
          <div
            key={layer.id}
            className="hero-art__layer"
            data-layer={layer.id}
            style={
              {
                left: pct(layer.x, canvas.w),
                top: pct(layer.y, canvas.h),
                width: pct(layer.w, canvas.w),
                height: pct(layer.h, canvas.h),
                '--i': i,
                '--hero-origin': layer.origin,
              } as CSSProperties
            }
          >
            <div className="hero-art__enter">
              <div className="hero-art__x">
                <div className="hero-art__y">
                  <div className="hero-art__event">
                    <div className="hero-art__idle">
                      <img
                        className="hero-art__img"
                        src={(dark && layer.srcDark) || layer.src}
                        alt=""
                        draggable={false}
                      />
                    </div>
                  </div>
                </div>
              </div>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
