// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState, type RefCallback } from 'react';
import { UI } from '../config/constants';
import { useMotion } from '../contexts/MotionContext';

interface EntrancePlayback {
  ref: RefCallback<HTMLElement>;
  playing: boolean;
}

/** Plays an entrance once, the first time the element is visible enough; reduced motion shows the final frame. */
export function useEntrancePlayback(threshold: number = UI.SETTINGS_HERO.PLAY_THRESHOLD): EntrancePlayback {
  const { reduced } = useMotion();
  const [element, setElement] = useState<HTMLElement | null>(null);
  const [seen, setSeen] = useState(false);

  useEffect(() => {
    if (!element || seen || reduced) return;
    if (typeof IntersectionObserver === 'undefined') {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- no observer to wait on, play at once
      setSeen(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting && e.intersectionRatio >= threshold)) {
          setSeen(true);
          observer.disconnect();
        }
      },
      { threshold }
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, [element, seen, reduced, threshold]);

  return { ref: setElement, playing: reduced || seen };
}
