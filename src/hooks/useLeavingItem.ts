// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useRef, useState } from 'react';
import { UI } from '../config';
import { useMotion } from '../contexts/MotionContext';

/** One item at a time plays its leave animation, then `done` runs; instant under reduced motion, dropped on unmount. */
export function useLeavingItem() {
  const { reduced } = useMotion();
  const [leaving, setLeaving] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  useEffect(() => () => clearTimeout(timerRef.current), []);

  const leave = useCallback(
    (id: string, done: () => void) => {
      if (reduced) {
        done();
        return;
      }
      setLeaving(id);
      timerRef.current = setTimeout(() => {
        setLeaving(null);
        done();
      }, UI.BOARDS_ROW.REMOVE_MS);
    },
    [reduced]
  );

  return { leaving, leave };
}
