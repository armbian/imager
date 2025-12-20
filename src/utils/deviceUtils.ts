/**
 * Device-related utility functions
 */

import type { BlockDevice } from '../types';
import type { DeviceType } from '../config/constants';

/**
 * Check if a device is still connected
 */
export function isDeviceConnected(devicePath: string, devices: BlockDevice[]): boolean {
  return devices.some(d => d.path === devicePath);
}

/**
 * Detect device type from BlockDevice properties
 */
export function getDeviceType(device: BlockDevice): DeviceType {
  if (device.is_system) {
    return 'system';
  }

  const busType = device.bus_type?.toLowerCase() || '';
  const path = device.path.toLowerCase();
  const model = device.model.toLowerCase();

  // Bus type detection (most reliable)
  if (busType === 'nvme' || busType.includes('nvme')) return 'nvme';
  if (busType === 'sas') return 'sas';
  if (busType === 'sata' || busType === 'ata') return 'sata';
  if (busType === 'usb') return 'usb';

  // Path-based detection
  if (path.includes('nvme')) return 'nvme';
  if (path.includes('mmcblk') || path.includes('mmc')) return 'sd';

  // Model-based detection
  if (model.includes('sd card') || model.includes('sdcard') || model.includes('mmc')) return 'sd';
  if (model.includes('ssd') || model.includes('nvme')) return 'nvme';
  if (model.includes('usb') || model.includes('flash')) return 'usb';

  // Fallback based on removability
  return device.is_removable ? 'usb' : 'hdd';
}
