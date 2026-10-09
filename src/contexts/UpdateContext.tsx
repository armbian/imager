// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { createContext, useContext, useState, useEffect, useCallback, useRef, type ReactNode } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch as relaunchProcess } from '@tauri-apps/plugin-process';
import { logInfo, logWarn } from '../hooks/useTauri';
import { getShowUpdaterModal } from '../hooks/useSettings';

export type UpdateCheckState = 'unchecked' | 'done' | 'failed';

interface UpdateContextType {
  /** Pending update, or null when none is available. */
  update: Update | null;
  /** True when an update is available to install. */
  available: boolean;
  /** Whether the update dialog is open. */
  isOpen: boolean;
  open: () => void;
  close: () => void;
  /** A simulated update clears itself instead of restarting the app */
  relaunch: () => Promise<void>;
  simulate?: (update: Update | null) => void;
  /** Checks again on demand, even when update notifications are off */
  recheck: () => Promise<void>;
  checking: boolean;
  /** Launch check skipped (notifications off) stays 'unchecked' until a recheck */
  checkState: UpdateCheckState;
}

const UpdateContext = createContext<UpdateContextType | undefined>(undefined);

// Checks for an update once on launch and shares it with the sidebar entry and the
// dialog. The dialog never auto-opens; the user opens it from the sidebar entry.
export function UpdateProvider({ children }: { children: ReactNode }) {
  const [update, setUpdate] = useState<Update | null>(null);
  const [isOpen, setIsOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkState, setCheckState] = useState<UpdateCheckState>('unchecked');
  const hasCheckedRef = useRef(false);
  const simulatedRef = useRef(false);

  const runCheck = useCallback(async () => {
    setChecking(true);
    try {
      const result = await check();
      setCheckState('done');
      // A simulated update stays until the dev panel clears it
      if (simulatedRef.current) return;
      setUpdate(result);
      logInfo(
        'updater',
        result ? `Update available: ${result.currentVersion} -> ${result.version}` : 'No updates available'
      );
    } catch (err) {
      setCheckState('failed');
      logWarn('updater', `Failed to check for updates: ${err}`);
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    if (hasCheckedRef.current) return;
    hasCheckedRef.current = true;

    const run = async () => {
      const notify = await getShowUpdaterModal();
      if (!notify) {
        logInfo('updater', 'Update notifications disabled in settings, skipping check');
        return;
      }
      await runCheck();
    };

    run();
  }, [runCheck]);

  const open = useCallback(() => setIsOpen(true), []);
  const close = useCallback(() => setIsOpen(false), []);

  const relaunch = useCallback(async () => {
    if (!simulatedRef.current) {
      await relaunchProcess();
      return;
    }
    logInfo('updater', 'Simulated update installed, relaunch skipped');
    simulatedRef.current = false;
    setUpdate(null);
    setIsOpen(false);
  }, []);

  const simulate = useCallback((next: Update | null) => {
    simulatedRef.current = next !== null;
    setUpdate(next);
    setIsOpen(false);
  }, []);

  return (
    <UpdateContext.Provider
      value={{
        update,
        available: update !== null,
        isOpen,
        open,
        close,
        relaunch,
        simulate: __DEV_SCENARIOS__ ? simulate : undefined,
        recheck: runCheck,
        checking,
        checkState,
      }}
    >
      {children}
    </UpdateContext.Provider>
  );
}

/** Access the update context (throws if used outside UpdateProvider) */
// eslint-disable-next-line react-refresh/only-export-components -- This is a hook, not a component
export function useUpdate(): UpdateContextType {
  const context = useContext(UpdateContext);
  if (!context) {
    throw new Error('useUpdate must be used within an UpdateProvider');
  }
  return context;
}
