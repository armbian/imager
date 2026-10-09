// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState } from 'react';

/** False while loading and for two frames after, so the first loaded value paints without a transition. */
export function useSettled(loading: boolean): boolean {
  const [settled, setSettled] = useState(!loading);

  useEffect(() => {
    if (loading) {
      // eslint-disable-next-line react-hooks/set-state-in-effect -- re-arms the guard when loading restarts
      setSettled(false);
      return;
    }
    let second = 0;
    const first = requestAnimationFrame(() => {
      second = requestAnimationFrame(() => setSettled(true));
    });
    return () => {
      cancelAnimationFrame(first);
      cancelAnimationFrame(second);
    };
  }, [loading]);

  return settled;
}
