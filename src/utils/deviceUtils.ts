import type { BlockDevice, QdlDevice } from '../types';
import type { DeviceType } from '../config/constants';

/** Check if device lists are different (by comparing paths and sizes). */
export function devicesChanged(prev: BlockDevice[] | null, next: BlockDevice[]): boolean {
  if (!prev) return true;
  if (prev.length !== next.length) return true;
  const prevKeys = new Set(prev.map((d) => `${d.path}:${d.size}`));
  return next.some((d) => !prevKeys.has(`${d.path}:${d.size}`));
}

/** Sort devices: system first, then by size descending. */
export function sortDevices(devices: BlockDevice[]): BlockDevice[] {
  return [...devices].sort((a, b) => {
    if (a.is_system !== b.is_system) return a.is_system ? -1 : 1;
    return b.size - a.size;
  });
}

/** Map a QDL device onto BlockDevice so it flows through the same selection UI. */
export function qdlToBlockDevice(qdl: QdlDevice): BlockDevice {
  return {
    path: `qdl://${qdl.bus_id}/${qdl.device_address}`,
    name: `Bus ${qdl.bus_id} Addr ${qdl.device_address}`,
    size: 0,
    size_formatted: '',
    model: 'Qualcomm Emergency Download',
    is_removable: true,
    is_system: false,
    bus_type: 'USB',
    is_read_only: false,
  };
}

/** Check if a device is still in the connected list */
export function isDeviceConnected(devicePath: string, devices: BlockDevice[]): boolean {
  return devices.some(d => d.path === devicePath);
}

/** Detect the device type from a BlockDevice's properties */
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
  if (busType === 'sd') return 'sd';

  // Path-based detection
  if (path.includes('nvme')) return 'nvme';
  if (path.includes('mmcblk') || path.includes('mmc')) return 'sd';

  // Model-based detection for SD cards (check before USB to prioritize SD detection)
  if (model.includes('sd card') || model.includes('sdcard') || model.includes('mmc') ||
      model.includes('sdxc') || model.includes('sdhc') || model.includes('sd reader')) return 'sd';
  if (model.includes('ssd') || model.includes('nvme')) return 'nvme';
  if (busType === 'usb') return 'usb';
  if (model.includes('usb') || model.includes('flash')) return 'usb';

  // Fallback based on removability
  return device.is_removable ? 'usb' : 'hdd';
}
