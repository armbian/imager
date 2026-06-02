import { useState, useEffect, useMemo } from 'react';
import { ArrowRight } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getBoards, getCachedBoardImage } from '../../hooks/useTauri';
import { shuffle } from '../../utils';
import { BoardImage } from '../shared';
import type { BoardInfo } from '../../types';

interface WelcomePageProps {
  /** Enter the selection flow. */
  onStart: () => void;
}

/** Only showcase well-supported boards on the landing. */
const TIERS = new Set(['platinum', 'gold', 'silver']);

/** How many boards flank the focused one on each side of the coverflow. */
const SIDE = 2;
/** Auto-advance cadence in milliseconds. */
const ADVANCE_MS = 3600;
/** Non-breaking space, used to keep the product name on a single line. */
const NBSP = String.fromCharCode(160);

/** Landing screen: an auto-advancing coverflow of well-supported boards, plus a "Start now" CTA. */
export function WelcomePage({ onStart }: WelcomePageProps) {
  const { t } = useTranslation();
  const [pool, setPool] = useState<BoardInfo[]>([]);
  const [images, setImages] = useState<Record<string, string | null>>({});
  const [index, setIndex] = useState(0);

  // Fetch the catalog once, keep only top-tier boards, shuffle a generous pool.
  useEffect(() => {
    let alive = true;
    getBoards()
      .then((boards) => {
        if (!alive) return;
        const top = boards.filter((b) => TIERS.has((b.support_tier || '').toLowerCase()));
        setPool(shuffle(top).slice(0, 24));
      })
      .catch(() => {
        // Offline or API error: the hero falls back to text only.
      });
    return () => {
      alive = false;
    };
  }, []);

  // Resolve photos for the pool (cached/remote); boards without one are skipped.
  useEffect(() => {
    let alive = true;
    if (pool.length === 0) return;
    Promise.all(
      pool.map((b) =>
        getCachedBoardImage(b.slug)
          .then((uri) => [b.slug, uri] as const)
          .catch(() => [b.slug, null] as const)
      )
    ).then((entries) => {
      if (alive) setImages(Object.fromEntries(entries));
    });
    return () => {
      alive = false;
    };
  }, [pool]);

  // Only boards that actually resolved a photo make it into the deck.
  const deck = useMemo(() => pool.filter((b) => images[b.slug]), [pool, images]);

  // Auto-advance the focused board, honoring the reduced-motion preference.
  const deckLen = deck.length;
  useEffect(() => {
    if (deckLen <= 1) return;
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    if (reduce) return;
    const id = setInterval(() => {
      setIndex((i) => (i + 1) % deckLen);
    }, ADVANCE_MS);
    return () => clearInterval(id);
  }, [deckLen]);

  // NBSP-bind "Armbian Imager" so the title wraps before the product name, not mid-name.
  const heading = t('home.welcomeHeading').replace(/Armbian\s*Imager/u, `Armbian${NBSP}Imager`);

  return (
    <div className="welcome">
      {deckLen > 0 && (
        <div className="welcome__stage" aria-hidden="true">
          {deck.map((board, i) => {
            // Signed shortest distance from the focused board on the ring.
            let off = i - index;
            if (off > deckLen / 2) off -= deckLen;
            if (off < -deckLen / 2) off += deckLen;
            const abs = Math.abs(off);
            // Slides beyond the visible flank are parked off-stage.
            if (abs > SIDE) return null;
            const scale = abs === 0 ? 1 : abs === 1 ? 0.72 : 0.5;
            const opacity = abs === 0 ? 1 : abs === 1 ? 0.55 : 0.22;
            const blur = abs === 0 ? 0 : abs === 1 ? 1.5 : 3;
            const style = {
              '--x': off,
              '--s': scale,
              opacity,
              filter: `blur(${blur}px)`,
              zIndex: 10 - abs,
            } as React.CSSProperties;
            return (
              <div
                key={board.slug}
                className={`welcome__slide ${abs === 0 ? 'is-focus' : ''}`}
                style={style}
              >
                <BoardImage src={images[board.slug]} alt="" />
              </div>
            );
          })}
        </div>
      )}

      <div className="welcome__content">
        <h1 className="welcome__heading">{heading}</h1>
        <p className="welcome__intro">{t('home.welcomeIntro')}</p>
        <button type="button" className="welcome__cta" onClick={onStart}>
          {t('home.startNow')}
          <ArrowRight size={18} />
        </button>
      </div>
    </div>
  );
}
