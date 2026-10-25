// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useRef, useState, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';
import { EVENTS, TIMING, UI } from '../config';
import { focusAfterInput, sleep } from '../utils';
import { classifyLoadError } from '../utils/errorUtils';
import { useToasts } from './useToasts';

const PANEL_FOCUS_TARGET = 'input, button:not(:disabled)';

interface ErrorRetry {
  error: string | null;
  retrying: boolean;
  /** `byPointer` suppresses the keyboard ring on restored focus. */
  retry: (byPointer: boolean) => Promise<void>;
  panelRef: RefObject<HTMLDivElement | null>;
}

interface RetryAttempt {
  restoreFocus: boolean;
  byPointer: boolean;
}

/** Retries a `useAsyncData` error in place, keeping the error view mounted and busy. */
export function useErrorRetry(
  error: string | null,
  loading: boolean,
  reload: () => Promise<unknown>
): ErrorRetry {
  const { t } = useTranslation();
  const { showError } = useToasts();
  const [retrying, setRetrying] = useState(false);
  const [heldError, setHeldError] = useState<string | null>(null);
  const busyRef = useRef(false);
  const mountedRef = useRef(true);
  const attemptRef = useRef<RetryAttempt | null>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  // Settles the attempt once, so a language change or a later render never repeats its toast or focus move.
  useEffect(() => {
    const attempt = attemptRef.current;
    if (!attempt || retrying || loading) return;
    attemptRef.current = null;
    if (error !== null) {
      showError(t('errorDisplay.stillFailing'));
      return;
    }
    const active = document.activeElement;
    const focusLost = !active || active === document.body || !active.isConnected;
    if (!attempt.restoreFocus || !focusLost) return;
    const panel = panelRef.current;
    const target = panel?.querySelector<HTMLElement>(PANEL_FOCUS_TARGET);
    if (target) focusAfterInput(target, attempt.byPointer);
  }, [retrying, loading, error, showError, t]);

  const retry = useCallback(
    async (byPointer: boolean) => {
      if (busyRef.current) return;
      busyRef.current = true;
      try {
        const inErrorView = !!document.activeElement?.closest(`.${UI.PANEL_ERROR_CLASS}`);
        attemptRef.current = { restoreFocus: inErrorView, byPointer };
        setHeldError(error);
        setRetrying(true);
        // A truly offline machine belongs on the offline screen.
        if (error && classifyLoadError(error) === 'network') {
          window.dispatchEvent(new Event(EVENTS.CONNECTIVITY_RECHECK));
        }
        await Promise.all([reload(), sleep(TIMING.RETRY_MIN_BUSY)]);
      } finally {
        busyRef.current = false;
        if (mountedRef.current) {
          setRetrying(false);
          setHeldError(null);
        }
      }
    },
    [error, reload]
  );

  return {
    error: error ?? (retrying ? heldError : null),
    retrying,
    retry,
    panelRef,
  };
}
