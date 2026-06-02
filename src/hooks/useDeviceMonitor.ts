import { useEffect, useCallback } from 'react';
import { getBlockDevices, getQdlDevices } from './useTauri';
import { POLLING } from '../config';
import type { BlockDevice } from '../types';

/** Monitor the selected device and clear it if it disconnects */
export function useDeviceMonitor(
  selectedDevice: BlockDevice | null,
  onDeviceDisconnected: () => void,
  enabled: boolean = true
) {
  const checkDevice = useCallback(async () => {
    if (!selectedDevice) return;

    try {
      // QDL (Qualcomm EDL) targets are not block devices; check the QDL list
      // instead, matching useFlashOperation's "gone when none present" semantics.
      if (selectedDevice.path.startsWith('qdl://')) {
        const qdlDevices = await getQdlDevices();
        if (qdlDevices.length === 0) {
          onDeviceDisconnected();
        }
        return;
      }

      const devices = await getBlockDevices();
      const stillConnected = devices.some(d => d.path === selectedDevice.path);

      if (!stillConnected) {
        onDeviceDisconnected();
      }
    } catch {
      // Silently ignore polling errors
    }
  }, [selectedDevice, onDeviceDisconnected]);

  useEffect(() => {
    if (!enabled || !selectedDevice) return;

    checkDevice();

    const interval = setInterval(checkDevice, POLLING.DEVICE_CHECK);
    return () => clearInterval(interval);
  }, [enabled, selectedDevice, checkDevice]);
}
