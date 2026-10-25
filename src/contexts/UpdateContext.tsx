// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { createContext, useContext, useState, useEffect, useCallback, useMemo, useRef, type ReactNode } from 'react';
import { useTranslation } from 'react-i18next';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch as relaunchProcess } from '@tauri-apps/plugin-process';
import { isAppInApplications, logInfo, logWarn } from '../hooks/useTauri';
import { getShowUpdaterModal } from '../hooks/useSettings';
import { useToasts } from '../hooks/useToasts';
import { UPDATER, type UpdateFailure } from '../config/constants';

export type UpdateCheckState = 'unchecked' | 'done' | 'failed';
export type UpdateStatus = 'available' | 'downloading' | 'ready' | 'installing' | 'error';

export interface UpdateProgress {
  downloaded: number;
  total: number | null;
}

interface Failure {
  step: UpdateFailure;
  key: string;
}

interface UpdateContextType {
  update: Update | null;
  available: boolean;
  status: UpdateStatus;
  progress: UpdateProgress;
  errorMessage: string | null;
  failedAt: UpdateFailure | null;
  /** True while a flash runs: restart and install retries are refused */
  restartBlocked: boolean;
  setRestartBlocked: (blocked: boolean) => void;
  download: () => Promise<void>;
  restart: () => Promise<void>;
  retry: () => Promise<void>;
  simulate?: (update: Update | null) => void;
  /** Checks again on demand, even when update notifications are off */
  recheck: () => Promise<void>;
  checking: boolean;
  /** Launch check skipped (notifications off) stays 'unchecked' until a recheck */
  checkState: UpdateCheckState;
}

const EMPTY_PROGRESS: UpdateProgress = { downloaded: 0, total: null };

const UpdateContext = createContext<UpdateContextType | undefined>(undefined);

// Owns the update lifecycle; install() quits the app on Windows, so nothing installs before restart.
export function UpdateProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const { showSuccess, showError } = useToasts();
  const [update, setUpdate] = useState<Update | null>(null);
  const [status, setStatus] = useState<UpdateStatus>('available');
  const [progress, setProgress] = useState<UpdateProgress>(EMPTY_PROGRESS);
  const [failure, setFailure] = useState<Failure | null>(null);
  const [restartBlocked, setRestartBlockedState] = useState(false);
  const [checking, setChecking] = useState(false);
  const [checkState, setCheckState] = useState<UpdateCheckState>('unchecked');
  const hasCheckedRef = useRef(false);
  const simulatedRef = useRef(false);
  const updateRef = useRef<Update | null>(null);
  const statusRef = useRef<UpdateStatus>('available');
  const failedAtRef = useRef<UpdateFailure | null>(null);
  const restartBlockedRef = useRef(false);
  const downloadingRef = useRef<Update | null>(null);
  const installingRef = useRef<Update | null>(null);
  const installedRef = useRef<Update | null>(null);

  const changeStatus = useCallback((next: UpdateStatus, nextFailure: Failure | null = null) => {
    statusRef.current = next;
    failedAtRef.current = nextFailure?.step ?? null;
    setStatus(next);
    setFailure(nextFailure);
  }, []);

  const adopt = useCallback(
    (next: Update | null) => {
      updateRef.current = next;
      setUpdate(next);
      setProgress(EMPTY_PROGRESS);
      changeStatus('available');
    },
    [changeStatus]
  );

  const runCheck = useCallback(async () => {
    setChecking(true);
    try {
      const result = await check();
      setCheckState('done');
      // A simulated update stays until the dev panel clears it
      if (simulatedRef.current) return;
      // Replacing the handle would drop a running or finished download
      const busy = statusRef.current !== 'available' && failedAtRef.current !== 'download';
      if (updateRef.current && busy) return;
      adopt(result);
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
  }, [adopt]);

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

  const setRestartBlocked = useCallback((blocked: boolean) => {
    restartBlockedRef.current = blocked;
    setRestartBlockedState(blocked);
  }, []);

  const download = useCallback(async () => {
    const target = updateRef.current;
    if (!target || downloadingRef.current === target) return;
    const canStart =
      statusRef.current === 'available' || (statusRef.current === 'error' && failedAtRef.current === 'download');
    if (!canStart) return;

    downloadingRef.current = target;
    changeStatus('downloading');
    setProgress(EMPTY_PROGRESS);
    logInfo('updater', `Downloading update ${target.version}`);

    let downloaded = 0;
    let total: number | null = null;
    let flushedAt = 0;
    const flush = () => {
      flushedAt = Date.now();
      if (updateRef.current === target) setProgress({ downloaded, total });
    };

    try {
      await target.download((event) => {
        switch (event.event) {
          case 'Started':
            total = event.data.contentLength ?? null;
            flush();
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (Date.now() - flushedAt >= UPDATER.PROGRESS_FLUSH_MS) flush();
            break;
          case 'Finished':
            // Fires before the signature check; ready waits for the promise
            flush();
            break;
        }
      });
      if (updateRef.current !== target) return;
      logInfo('updater', `Update ${target.version} downloaded, signature verified`);
      changeStatus('ready');
      showSuccess(t('update.readyMessage'));
    } catch (err) {
      if (updateRef.current !== target) return;
      logWarn('updater', `Update download failed: ${err}`);
      changeStatus('error', { step: 'download', key: UPDATER.ERROR_KEY.download });
      showError(t(UPDATER.ERROR_KEY.download));
    } finally {
      if (downloadingRef.current === target) downloadingRef.current = null;
    }
  }, [changeStatus, showSuccess, showError, t]);

  const installFailure = useCallback(async (): Promise<Failure> => {
    if (!simulatedRef.current) {
      try {
        if (!(await isAppInApplications())) {
          return { step: 'install', key: UPDATER.NOT_IN_APPLICATIONS_KEY };
        }
      } catch {
        // Fall through to the install error itself
      }
    }
    return { step: 'install', key: UPDATER.ERROR_KEY.install };
  }, []);

  const restart = useCallback(async () => {
    const target = updateRef.current;
    if (!target || installingRef.current === target) return;
    const s = statusRef.current;
    const f = failedAtRef.current;
    if (s !== 'ready' && !(s === 'error' && (f === 'install' || f === 'relaunch'))) return;
    if (restartBlockedRef.current) {
      logWarn('updater', 'Restart refused: flash in progress');
      return;
    }

    installingRef.current = target;
    changeStatus('installing');
    try {
      if (installedRef.current !== target) {
        logInfo('updater', `Installing update ${target.version}`);
        try {
          await target.install();
        } catch (err) {
          if (updateRef.current !== target) return;
          logWarn('updater', `Update install failed: ${err}`);
          changeStatus('error', await installFailure());
          return;
        }
        installedRef.current = target;
      }
      if (updateRef.current !== target) return;
      // install() can sit on a password prompt long enough for a flash to start meanwhile
      if (restartBlockedRef.current) {
        logWarn('updater', 'Relaunch deferred: flash in progress');
        changeStatus('ready');
        return;
      }

      if (simulatedRef.current) {
        logInfo('updater', 'Simulated update installed, relaunch skipped');
        simulatedRef.current = false;
        adopt(null);
        return;
      }

      try {
        await relaunchProcess();
      } catch (err) {
        logWarn('updater', `Relaunch after update failed: ${err}`);
        changeStatus('error', { step: 'relaunch', key: UPDATER.ERROR_KEY.relaunch });
      }
    } finally {
      if (installingRef.current === target) installingRef.current = null;
    }
  }, [adopt, changeStatus, installFailure]);

  const retry = useCallback(async () => {
    if (statusRef.current !== 'error') return;
    if (failedAtRef.current === 'download') return download();
    return restart();
  }, [download, restart]);

  const simulate = useCallback(
    (next: Update | null) => {
      simulatedRef.current = next !== null;
      adopt(next);
    },
    [adopt]
  );

  const errorMessage = failure ? t(failure.key) : null;

  const value = useMemo<UpdateContextType>(
    () => ({
      update,
      available: update !== null,
      status,
      progress,
      errorMessage,
      failedAt: failure?.step ?? null,
      restartBlocked,
      setRestartBlocked,
      download,
      restart,
      retry,
      simulate: __DEV_SCENARIOS__ ? simulate : undefined,
      recheck: runCheck,
      checking,
      checkState,
    }),
    [
      update,
      status,
      progress,
      errorMessage,
      failure,
      restartBlocked,
      setRestartBlocked,
      download,
      restart,
      retry,
      simulate,
      runCheck,
      checking,
      checkState,
    ]
  );

  return <UpdateContext.Provider value={value}>{children}</UpdateContext.Provider>;
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
