// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useLayoutEffect, useRef, useState, type RefObject } from 'react';

const TABBABLE = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled]):not([type="hidden"])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  '[tabindex]:not([tabindex="-1"])',
].join(',');

// Only the topmost open dialog traps keys, so a dialog opened from another one stays usable.
const openDialogs: HTMLElement[] = [];

function tabbables(root: HTMLElement): HTMLElement[] {
  return Array.from(root.querySelectorAll<HTMLElement>(TABBABLE)).filter((el) => el.getClientRects().length > 0);
}

interface DialogFocusOptions {
  onEscape?: () => void;
  returnFocusRef?: RefObject<HTMLElement | null>;
}

/** Moves focus into the dialog, keeps Tab inside it, and gives focus back to the opener when it unmounts. */
export function useDialogFocus<T extends HTMLElement>({ onEscape, returnFocusRef }: DialogFocusOptions = {}): RefObject<T | null> {
  const dialogRef = useRef<T | null>(null);
  // Read during render: an autoFocus field inside takes focus before any effect runs.
  const [opener] = useState(() => (document.activeElement instanceof HTMLElement ? document.activeElement : null));
  const onEscapeRef = useRef(onEscape);
  useLayoutEffect(() => {
    onEscapeRef.current = onEscape;
  }, [onEscape]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;
    const returnTo = returnFocusRef?.current ?? opener;
    openDialogs.push(dialog);
    if (!dialog.contains(document.activeElement)) (tabbables(dialog)[0] ?? dialog).focus();

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || openDialogs[openDialogs.length - 1] !== dialog) return;
      if (event.key === 'Escape') {
        if (!onEscapeRef.current) return;
        event.preventDefault();
        onEscapeRef.current();
        return;
      }
      if (event.key !== 'Tab') return;
      const items = tabbables(dialog);
      const first = items[0];
      const last = items[items.length - 1];
      const active = document.activeElement;
      if (!first) {
        event.preventDefault();
        return;
      }
      if (!dialog.contains(active)) {
        event.preventDefault();
        (event.shiftKey ? last : first).focus();
      } else if (event.shiftKey && (active === first || active === dialog)) {
        event.preventDefault();
        last.focus();
      } else if (!event.shiftKey && active === last) {
        event.preventDefault();
        first.focus();
      }
    };
    document.addEventListener('keydown', onKeyDown);

    return () => {
      document.removeEventListener('keydown', onKeyDown);
      openDialogs.splice(openDialogs.indexOf(dialog), 1);
      // StrictMode replays effects with the dialog still mounted; only a real close hands focus back.
      if (dialog.isConnected) return;
      if (returnTo?.isConnected) returnTo.focus();
    };
  }, [opener, returnFocusRef]);

  return dialogRef;
}
