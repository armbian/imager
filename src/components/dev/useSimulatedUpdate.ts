// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import { useCallback } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { SimulatedUpdate } from './SimulatedUpdate';
import { useUpdate } from '../../contexts/UpdateContext';

export function useSimulatedUpdate() {
  const { update, simulate } = useUpdate();
  const simulated = update instanceof SimulatedUpdate ? update : null;

  const offer = useCallback(
    async (fails: boolean) => {
      if (!simulate) return;
      await simulated?.close();
      simulate(new SimulatedUpdate(await getVersion(), fails));
    },
    [simulate, simulated]
  );

  const clear = useCallback(async () => {
    try {
      await simulated?.close();
    } finally {
      simulate?.(null);
    }
  }, [simulate, simulated]);

  return { simulated, canSimulate: simulate !== undefined, offer, clear };
}
