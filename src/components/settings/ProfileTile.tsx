// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState, type CSSProperties, type SyntheticEvent } from 'react';
import { useTranslation } from 'react-i18next';
import { UI } from '../../config';
import { ALL_BOARDS_ART } from '../../config/heroArt';
import { useMotion } from '../../contexts/MotionContext';
import type { ProfileBoard } from '../../types';
import { BoardImage } from '../shared';
import { useBoardPhotos } from '../../hooks/useBoardPhotos';
import { opaqueFitTransform } from '../../utils/imageFit';

interface ProfileTileProps {
  boards: ProfileBoard[];
  /** Square side in px; omitted, the surrounding layout sizes the tile */
  size?: number;
  /** Slides automatically unless false, so a multi-board profile reads as such at a glance */
  autoplay?: boolean;
}

/** Board art for a profile: all-boards art, one photo, or a sideways carousel that slides on its own. */
export function ProfileTile({ boards, size, autoplay = true }: ProfileTileProps) {
  const { t } = useTranslation();
  const { reduced } = useMotion();
  const count = boards.length;
  const slugs = boards.map((b) => b.slug);
  const slugKey = slugs.join(' ');
  const photos = useBoardPhotos(slugs);
  // Per photo source: the transform that fits its visible board, or null once known to have none
  const [fits, setFits] = useState<Record<string, string | null>>({});

  // pos === count is the clone of the first slide, so the forward loop keeps sliding the same way
  const [pos, setPos] = useState(0);
  const [instant, setInstant] = useState(false);
  const [lastKey, setLastKey] = useState(slugKey);
  if (slugKey !== lastKey) {
    setLastKey(slugKey);
    setInstant(true);
    setPos(0);
  }
  const isCarousel = count > 1;
  const active = isCarousel ? pos % count : 0;

  useEffect(() => {
    if (!isCarousel || !autoplay || reduced) return;
    const timer = setInterval(() => {
      if (document.hidden) return;
      setInstant(false);
      setPos((p) => {
        const cur = p % count;
        return cur < count - 1 ? cur + 1 : count;
      });
    }, UI.PROFILES.CAROUSEL_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [isCarousel, autoplay, reduced, count]);

  useEffect(() => {
    if (pos !== count || !isCarousel) return;
    const snap = setTimeout(() => {
      setInstant(true);
      setPos(0);
    }, UI.PROFILES.CAROUSEL_SLIDE_MS);
    return () => clearTimeout(snap);
  }, [pos, count, isCarousel]);

  const style = size ? { width: size, height: size } : undefined;
  const fit = (event: SyntheticEvent<HTMLImageElement>) => {
    const img = event.currentTarget;
    const transform = opaqueFitTransform(img, UI.PROFILES.TILE_FILL);
    const src = img.getAttribute('src') ?? '';
    setFits((prev) => (prev[src] === transform ? prev : { ...prev, [src]: transform }));
  };
  const photo = (index: number, alt: string) => {
    const slug = boards[index].slug;
    if (!(slug in photos)) return null;
    const src = photos[slug];
    const known = !!src && src in fits;
    const transform = known ? fits[src] : null;
    const className = `profile-tile__img${known ? ' is-measured' : ''}${transform ? ' is-fit' : ''}`;
    return (
      <span className="profile-tile__frame" style={transform ? ({ '--tile-fit': transform } as CSSProperties) : undefined}>
        <BoardImage src={src} alt={alt} className={className} onLoad={fit} />
      </span>
    );
  };

  if (count === 0) {
    return (
      <span className="profile-tile profile-tile--all" style={style}>
        <img className="profile-tile__all" src={ALL_BOARDS_ART} alt={t('settings.profiles.allBoards')} />
      </span>
    );
  }

  if (!isCarousel) {
    return (
      <span className="profile-tile" style={style}>
        {photo(0, boards[0].name)}
      </span>
    );
  }

  const slides = [...boards, boards[0]];
  return (
    <span className="profile-tile profile-tile--carousel" style={style}>
      <span className="profile-tile__viewport">
        <span
          className={`profile-tile__track${instant || reduced ? ' is-instant' : ''}`}
          style={{ transform: `translateX(${-pos * 100}%)` }}
        >
          {slides.map((b, i) => {
            const clone = i === count;
            return (
              <span key={clone ? 'clone' : b.slug} className="profile-tile__slide" aria-hidden={clone || i !== active}>
                {photo(clone ? 0 : i, clone ? '' : b.name)}
              </span>
            );
          })}
        </span>
      </span>
    </span>
  );
}
