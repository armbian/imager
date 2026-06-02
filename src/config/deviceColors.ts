/** Color configuration per storage device type for visual distinction */

import { PALETTE } from './constants';
import type { DeviceType } from './constants';

export interface DeviceColorConfig {
  /** Semi-transparent for a subtle effect */
  background: string;
  text: string;
  border?: string;
}

export const DEVICE_COLORS: Record<DeviceType, DeviceColorConfig> = {
  system: {
    background: 'rgba(239, 68, 68, 0.1)',
    text: PALETTE.RED,
    border: 'rgba(239, 68, 68, 0.3)',
  },
  sd: {
    background: 'rgba(59, 130, 246, 0.1)',
    text: PALETTE.BLUE,
    border: 'rgba(59, 130, 246, 0.3)',
  },
  usb: {
    background: 'rgba(16, 185, 129, 0.1)',
    text: PALETTE.GREEN,
    border: 'rgba(16, 185, 129, 0.3)',
  },
  sata: {
    background: 'rgba(249, 115, 22, 0.1)',
    text: '#f97316',
    border: 'rgba(249, 115, 22, 0.3)',
  },
  sas: {
    background: 'rgba(168, 85, 247, 0.1)',
    text: '#a855f7',
    border: 'rgba(168, 85, 247, 0.3)',
  },
  nvme: {
    background: 'rgba(236, 72, 153, 0.1)',
    text: '#ec4899',
    border: 'rgba(236, 72, 153, 0.3)',
  },
  hdd: {
    background: 'var(--bg-secondary)',
    text: 'var(--text-secondary)',
    border: 'var(--border-color)',
  },
};

/** Get color config for a device type with fallback */
export function getDeviceColors(deviceType: DeviceType): DeviceColorConfig {
  return DEVICE_COLORS[deviceType] || DEVICE_COLORS.hdd;
}
