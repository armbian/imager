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

/** External links */
export const LINKS = {
  /** GitHub repository URL */
  GITHUB_REPO: 'https://github.com/armbian/imager',
  /** Documentation URL */
  DOCS: 'https://docs.armbian.com',
  /** Community forum URL */
  FORUM: 'https://forum.armbian.com',
  /** MOTD (Message of the Day) JSON file */
  MOTD: 'https://raw.githubusercontent.com/armbian/os/main/motd.json',
} as const;

/** Message rotation intervals */
export const TIMING = {
  /** MOTD rotation interval in milliseconds */
  MOTD_ROTATION: 30000, // 30 seconds
} as const;
