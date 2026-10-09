// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { EVENTS, TIMING } from '../config/constants';
import { useToasts } from './useToasts';
import { logWarn } from './useTauri';

interface SettingToggleOptions<T> {
  /** Shown when the stored value cannot be read, so the control is not left disabled */
  fallback: T;
  /** Toasts `${toastKey}Updated` / `${toastKey}Error` */
  toastKey?: string;
  event?: string;
}

// A store that fails fails for every toggle on the page: one toast covers them all
let lastLoadErrorAt = 0;

interface SettingToggle<T> {
  /** undefined until the first load settles, so controls can skip their mount animation */
  value: T | undefined;
  set: (next: T) => Promise<void>;
  busy: boolean;
}

/** One persisted setting with an optimistic write that rolls back and toasts on failure. */
export function useSettingToggle<T>(
  load: () => Promise<T>,
  save: (value: T) => Promise<void>,
  { fallback, toastKey, event = EVENTS.SETTINGS_CHANGED }: SettingToggleOptions<T>
): SettingToggle<T> {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();
  const [value, setValue] = useState<T | undefined>(undefined);
  const [busy, setBusy] = useState(false);
  const loadRef = useRef(load);
  const saveRef = useRef(save);
  const busyRef = useRef(false);
  const valueRef = useRef<T | undefined>(undefined);
  const fallbackRef = useRef(fallback);

  useEffect(() => {
    loadRef.current = load;
    saveRef.current = save;
    fallbackRef.current = fallback;
  });

  useEffect(() => {
    let cancelled = false;
    loadRef
      .current()
      .then((loaded) => {
        if (cancelled) return;
        valueRef.current = loaded;
        setValue(loaded);
      })
      .catch((error) => {
        logWarn('settings', `Failed to load setting: ${error}`);
        if (cancelled) return;
        valueRef.current = fallbackRef.current;
        setValue(fallbackRef.current);
        const now = Date.now();
        if (now - lastLoadErrorAt < TIMING.TOAST_DURATION) return;
        lastLoadErrorAt = now;
        showError(t('settings.toast.loadError'));
      });
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- loads once on mount
  }, []);

  const set = useCallback(
    async (next: T) => {
      if (busyRef.current) return;
      busyRef.current = true;
      setBusy(true);
      const previous = valueRef.current;
      valueRef.current = next;
      setValue(next);

      try {
        await saveRef.current(next);
        window.dispatchEvent(new Event(event));
        if (toastKey) showSuccess(t(`${toastKey}Updated`));
      } catch (error) {
        logWarn('settings', `Failed to save setting: ${error}`);
        valueRef.current = previous;
        setValue(previous);
        showError(t(toastKey ? `${toastKey}Error` : 'settings.toast.saveError'));
      } finally {
        busyRef.current = false;
        setBusy(false);
      }
    },
    [event, toastKey, showSuccess, showError, t]
  );

  return { value, set, busy };
}
