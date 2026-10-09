// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

// Arrow, Home, End, Enter, Escape and Tab for a listbox that keeps focus on one element (aria-activedescendant).

import { useState, type KeyboardEvent } from 'react';

interface ListboxKeys {
  active: number;
  setActive: (index: number) => void;
  onKeyDown: (event: KeyboardEvent<HTMLElement>) => void;
}

export function useListboxKeys(
  count: number,
  onPick: (index: number) => void,
  onClose: () => void,
  initial = 0
): ListboxKeys {
  const [rawActive, setActive] = useState(initial);
  const active = count === 0 ? -1 : Math.min(Math.max(rawActive, 0), count - 1);

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    const move = (next: number) => {
      event.preventDefault();
      if (count > 0) setActive((next + count) % count);
    };
    switch (event.key) {
      case 'ArrowDown':
        move(active + 1);
        break;
      case 'ArrowUp':
        move(active - 1);
        break;
      case 'Home':
        move(0);
        break;
      case 'End':
        move(count - 1);
        break;
      case 'Enter':
        event.preventDefault();
        if (active >= 0) onPick(active);
        break;
      case 'Escape':
        event.preventDefault();
        event.stopPropagation();
        onClose();
        break;
      case 'Tab':
        // Closing refocuses the trigger, so the browser's own Tab moves on from there.
        onClose();
        break;
    }
  }

  return { active, setActive, onKeyDown };
}
