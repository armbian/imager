/**
 * Application constants and configuration values
 */

/** Polling intervals in milliseconds */
export const POLLING = {
  /** Device connection check interval */
  DEVICE_CHECK: 2000,
  /** Download progress update interval */
  DOWNLOAD_PROGRESS: 250,
  /** Flash progress update interval */
  FLASH_PROGRESS: 250,
} as const;

/** Device type identifiers */
export type DeviceType = 'system' | 'sd' | 'usb' | 'sata' | 'sas' | 'nvme' | 'hdd';
