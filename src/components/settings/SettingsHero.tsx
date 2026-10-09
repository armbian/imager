// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useId, type CSSProperties, type ReactNode } from 'react';
import { UI } from '../../config';
import { HERO_ART } from '../../config/heroArt';
import { useEntrancePlayback } from '../../hooks/useEntrancePlayback';
import type { HeroId } from '../../types';
import { HeroArt } from './HeroArt';

interface SettingsHeroProps {
  hero: HeroId;
  title: string;
  lead: ReactNode;
  heroState?: Record<string, string>;
  actions?: ReactNode;
  meta?: ReactNode;
}

export function SettingsHero({ hero, title, lead, heroState, actions, meta }: SettingsHeroProps) {
  const titleId = useId();
  const { ref, playing } = useEntrancePlayback(UI.SETTINGS_HERO.PLAY_THRESHOLD);
  const { canvas } = HERO_ART[hero];

  return (
    <section
      ref={ref}
      className="settings-hero"
      style={{ '--hero-ratio': canvas.w / canvas.h } as CSSProperties}
      aria-labelledby={titleId}
    >
      <div className="settings-hero__text">
        <h1 id={titleId} className="settings-hero__title">
          {title}
        </h1>
        <p className="settings-hero__lead">{lead}</p>
        {meta && <div className="settings-hero__meta">{meta}</div>}
        {actions && <div className="settings-hero__actions">{actions}</div>}
      </div>
      <div className="settings-hero__art">
        <HeroArt hero={hero} playing={playing} state={heroState} />
      </div>
    </section>
  );
}
