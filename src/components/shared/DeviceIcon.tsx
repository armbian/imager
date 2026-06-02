import { HardDrive, Lock, MemoryStick, Usb } from 'lucide-react';
import type { DeviceType } from '../../config';

// Storage device icon picked by detected device type.
export function DeviceIcon({ type, size = 24 }: { type: DeviceType; size?: number }) {
  switch (type) {
    case 'system':
      return <Lock size={size} />;
    case 'sd':
      return <MemoryStick size={size} />;
    case 'usb':
      return <Usb size={size} />;
    case 'sata':
    case 'sas':
    case 'nvme':
    default:
      return <HardDrive size={size} />;
  }
}

// Translated badge label for a device type, or null when none applies.
// eslint-disable-next-line react-refresh/only-export-components -- icon helper paired with its component
export function getDeviceBadge(type: DeviceType, t: (key: string) => string): string | null {
  switch (type) {
    case 'system':
      return t('device.system');
    case 'sd':
      return t('device.sdCard');
    case 'usb':
      return t('device.usb');
    case 'sata':
      return t('device.sata');
    case 'sas':
      return t('device.sas');
    case 'nvme':
      return t('device.nvme');
    default:
      return null;
  }
}
