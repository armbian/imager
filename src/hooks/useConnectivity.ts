// Poll backend connectivity every 30s; starts optimistic (online) until the first check.
// When the force_offline setting is enabled, the hook reports offline regardless of real connectivity.

import { useState, useEffect, useRef, useCallback } from 'react';
import { checkConnectivity } from './useTauri';
import { getForceOffline } from './useSettings';
import { POLLING, EVENTS } from '../config';

interface ConnectivityState {
  /** Whether the app can reach the Armbian API */
  isOnline: boolean;
}

export function useConnectivity(): ConnectivityState {
  const [isOnline, setIsOnline] = useState(true); // Optimistic default
  const [forced, setForced] = useState(false); // force_offline setting
  const mountedRef = useRef(true);

  const check = useCallback(async () => {
    try {
      const online = await checkConnectivity();
      if (mountedRef.current) {
        setIsOnline(online);
      }
    } catch {
      if (mountedRef.current) {
        setIsOnline(false);
      }
    }
  }, []);

  const readForced = useCallback(async () => {
    try {
      const value = await getForceOffline();
      if (mountedRef.current) {
        setForced(value);
      }
    } catch {
      if (mountedRef.current) {
        setForced(false);
      }
    }
  }, []);

  // Track mount status once for both async reads to guard against late state updates
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    // Skip connectivity polling entirely while forced offline mode is active
    if (forced) return;

    // eslint-disable-next-line react-hooks/set-state-in-effect -- Initial connectivity check on mount
    check();

    const interval = setInterval(check, POLLING.CONNECTIVITY_CHECK);

    return () => clearInterval(interval);
  }, [check, forced]);

  useEffect(() => {
    // eslint-disable-next-line react-hooks/set-state-in-effect -- Initial force_offline read on mount
    readForced();

    // Re-read the setting whenever it changes elsewhere in the app
    window.addEventListener(EVENTS.SETTINGS_CHANGED, readForced);

    return () => {
      window.removeEventListener(EVENTS.SETTINGS_CHANGED, readForced);
    };
  }, [readForced]);

  return { isOnline: forced ? false : isOnline };
}
