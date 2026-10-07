// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

export function useWindowFullscreen(): boolean {
  const [fullscreen, setFullscreen] = useState(false);

  useEffect(() => {
    const win = getCurrentWindow();
    let cancelled = false;
    const check = () => {
      win
        .isFullscreen()
        .then((value) => {
          if (!cancelled) setFullscreen(value === true);
        })
        .catch(() => undefined);
    };
    check();
    const unlisten = win.onResized(check);
    return () => {
      cancelled = true;
      void unlisten.then((stop) => stop()).catch(() => undefined);
    };
  }, []);

  return fullscreen;
}
