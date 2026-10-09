// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState, type CSSProperties } from 'react';
import { HERO_ART, HERO_SETTLE_MS } from '../../config/heroArt';
import type { HeroId } from '../../types';

interface HeroArtProps {
  hero: HeroId;
  playing: boolean;
  state?: Record<string, string>;
}

const pct = (value: number, total: number) => `${(value / total) * 100}%`;

/** Layered hero illustration; `state` keys become data-* attributes the hero CSS reacts to. */
export function HeroArt({ hero, playing, state }: HeroArtProps) {
  const { canvas, layers } = HERO_ART[hero];
  const [settled, setSettled] = useState(false);

  useEffect(() => {
    if (!playing) return;
    const id = window.setTimeout(() => setSettled(true), HERO_SETTLE_MS);
    return () => window.clearTimeout(id);
  }, [playing]);

  const stateAttrs = Object.fromEntries(Object.entries(state ?? {}).map(([key, value]) => [`data-${key}`, value]));

  return (
    <div
      className={`hero-art hero-art--${hero}`}
      data-playing={playing}
      data-settled={settled || undefined}
      aria-hidden="true"
      {...stateAttrs}
    >
      <div className="hero-art__stack">
        {layers.map((layer, i) => (
          <img
            key={layer.id}
            className="hero-art__layer"
            data-layer={layer.id}
            src={layer.src}
            alt=""
            draggable={false}
            style={
              {
                left: pct(layer.x, canvas.w),
                top: pct(layer.y, canvas.h),
                width: pct(layer.w, canvas.w),
                height: pct(layer.h, canvas.h),
                '--i': i,
              } as CSSProperties
            }
          />
        ))}
      </div>
    </div>
  );
}
