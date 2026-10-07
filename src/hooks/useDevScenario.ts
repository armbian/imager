// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback, useEffect, useState } from 'react';
import { useAsyncData } from './useAsyncData';
import { useToasts } from './useToasts';
import {
  devCreateVdisk,
  devDeleteVdisk,
  devMakeTestImage,
  devReset,
  devScenariosStatus,
  devSetScenario,
} from './useTauri';
import { EVENTS } from '../config';
import { DEV_SCENARIOS } from '../config/devScenarios';
import { getErrorMessage } from '../utils';
import type { DevPreset, DevScenario, DevScenariosStatus, DevTestImage } from '../types';

export interface DevScenarioApi {
  status: DevScenariosStatus | null;
  busy: boolean;
  setScenario: (next: DevScenario, success?: string) => Promise<void>;
  patch: (edit: (current: DevScenario) => DevScenario) => Promise<void>;
  applyPreset: (preset: DevPreset) => Promise<void>;
  reset: () => Promise<void>;
  resetSilently: () => Promise<void>;
  createVdisk: (id: string, sizeMb: number) => Promise<boolean>;
  deleteVdisk: (id: string) => Promise<void>;
  makeTestImage: (sizeMb: number, unaligned: boolean) => Promise<DevTestImage | null>;
}

export function useDevScenario(): DevScenarioApi {
  const { showSuccess, showError } = useToasts();
  const { data: status, reload } = useAsyncData(devScenariosStatus, []);
  const [busy, setBusy] = useState(false);
  const available = status !== null;

  useEffect(() => {
    if (!available) return;
    const id = setInterval(reload, DEV_SCENARIOS.STATUS_POLL_MS);
    return () => clearInterval(id);
  }, [available, reload]);

  const run = useCallback(
    async <T>(action: () => Promise<T>, success?: string): Promise<T | null> => {
      setBusy(true);
      try {
        const result = await action();
        // API faults change what the connectivity check answers; do not wait for its next poll
        window.dispatchEvent(new Event(EVENTS.CONNECTIVITY_RECHECK));
        if (success) showSuccess(success);
        return result;
      } catch (err) {
        showError(getErrorMessage(err, 'Dev scenario change failed'));
        return null;
      } finally {
        await reload();
        setBusy(false);
      }
    },
    [reload, showSuccess, showError]
  );

  const setScenario = useCallback(
    async (next: DevScenario, success?: string) => {
      await run(() => devSetScenario(next), success);
    },
    [run]
  );

  const patch = useCallback(
    async (edit: (current: DevScenario) => DevScenario) => {
      if (!status) return;
      await setScenario(edit(status.scenario));
    },
    [status, setScenario]
  );

  const applyPreset = useCallback(
    (preset: DevPreset) => setScenario(preset.scenario, `Scenario: ${preset.label}`),
    [setScenario]
  );

  const reset = useCallback(async () => {
    await run(devReset, 'Simulation off: real devices, flashing and network');
  }, [run]);

  const resetSilently = useCallback(async () => {
    await run(devReset);
  }, [run]);

  const createVdisk = useCallback(
    async (id: string, sizeMb: number) =>
      (await run(() => devCreateVdisk(id, sizeMb), `Virtual disk ${id} created`)) !== null,
    [run]
  );

  const deleteVdisk = useCallback(
    async (id: string) => {
      await run(() => devDeleteVdisk(id), `Virtual disk ${id} deleted`);
    },
    [run]
  );

  const makeTestImage = useCallback(
    (sizeMb: number, unaligned: boolean) => run(() => devMakeTestImage(sizeMb, unaligned)),
    [run]
  );

  return { status, busy, setScenario, patch, applyPreset, reset, resetSilently, createVdisk, deleteVdisk, makeTestImage };
}
