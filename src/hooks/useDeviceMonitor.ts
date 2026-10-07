// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { useEffect, useCallback } from 'react';
import { isTargetConnected } from './useTauri';
import { POLLING } from '../config';
import type { BlockDevice } from '../types';

/** Clears the selected device once it disconnects; `edl` follows the image's flash method, never the path. */
export function useDeviceMonitor(
  selectedDevice: BlockDevice | null,
  edl: boolean,
  onDeviceDisconnected: () => void,
  enabled: boolean = true
) {
  const checkDevice = useCallback(async () => {
    if (!selectedDevice) return;

    try {
      if (!(await isTargetConnected(selectedDevice, edl))) {
        onDeviceDisconnected();
      }
    } catch {
      // Silently ignore polling errors
    }
  }, [selectedDevice, edl, onDeviceDisconnected]);

  useEffect(() => {
    if (!enabled || !selectedDevice) return;

    checkDevice();

    const interval = setInterval(checkDevice, POLLING.DEVICE_CHECK);
    return () => clearInterval(interval);
  }, [enabled, selectedDevice, checkDevice]);
}
