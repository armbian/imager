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

/** Timing constants in milliseconds */
export const TIMING = {
  /** MOTD rotation interval */
  MOTD_ROTATION: 30000,
  /** Duration to show "Copied!" notification */
  COPIED_NOTIFICATION: 2000,
} as const;

/** Cache configuration */
export const CACHE = {
  /** Maximum consecutive flash failures before auto-deleting cached image */
  MAX_FLASH_FAILURES: 3,
  /** Default maximum cache size: 20 GB */
  DEFAULT_SIZE: 20 * 1024 * 1024 * 1024,
  /** Cache size options in bytes with display labels */
  SIZE_OPTIONS: [
    { value: 5 * 1024 * 1024 * 1024, label: '5 GB' },
    { value: 10 * 1024 * 1024 * 1024, label: '10 GB' },
    { value: 20 * 1024 * 1024 * 1024, label: '20 GB' },
    { value: 50 * 1024 * 1024 * 1024, label: '50 GB' },
    { value: 100 * 1024 * 1024 * 1024, label: '100 GB' },
  ],
} as const;

/** Custom DOM events for inter-component communication */
export const EVENTS = {
  /** Fired when MOTD setting changes */
  MOTD_CHANGED: 'armbian-motd-changed',
  /** Fired when general settings change */
  SETTINGS_CHANGED: 'armbian-settings-changed',
} as const;

/** Storage key prefixes for sessionStorage/localStorage */
export const STORAGE_KEYS = {
  /** Prefix for flash failure count (appended with image URL) */
  FLASH_FAILURE_PREFIX: 'flash_failure_count_',
} as const;

/** Settings store configuration */
export const SETTINGS = {
  /** Settings file name */
  FILE: 'settings.json',
  /** Store key names */
  KEYS: {
    THEME: 'theme',
    LANGUAGE: 'language',
    SHOW_MOTD: 'show_motd',
    SHOW_UPDATER_MODAL: 'show_updater_modal',
    DEVELOPER_MODE: 'developer_mode',
    CACHE_ENABLED: 'cache_enabled',
    CACHE_MAX_SIZE: 'cache_max_size',
  },
  /** Default values for settings */
  DEFAULTS: {
    THEME: 'auto',
    LANGUAGE: 'en',
    SHOW_MOTD: true,
    SHOW_UPDATER_MODAL: true,
    DEVELOPER_MODE: false,
    CACHE_ENABLED: true,
  },
} as const;

/** UI color constants */
export const COLORS = {
  /** Default icon color (slate-500) */
  DEFAULT_ICON: '#64748b',
  /** Alert/warning icon color (amber-500) */
  ALERT_WARNING: '#f59e0b',
  /** QR code foreground color */
  QR_DARK: '#000000',
  /** QR code background color */
  QR_LIGHT: '#ffffff',
} as const;

/** QR code configuration */
export const QR_CODE = {
  /** Width in pixels */
  WIDTH: 120,
  /** Margin in modules */
  MARGIN: 1,
} as const;

/** UI dimension constants */
export const UI = {
  /** Skeleton placeholder counts */
  SKELETON: {
    BOARD_GRID_COUNT: 8,
    LIST_COUNT: 6,
    MANUFACTURER_MODAL: 6,
    DEVICE_MODAL: 4,
    IMAGE_MODAL: 6,
  },
  /** Marquee text configuration */
  MARQUEE: {
    DEFAULT_WIDTH: 180,
    SEPARATOR_WIDTH: 5,
  },
  /** Icon sizes in pixels */
  ICON_SIZE: {
    SEARCH: 18,
    FLASH_STAGE: 32,
  },
} as const;

/** Vendor/manufacturer constants */
export const VENDOR = {
  /** Fallback vendor ID for boards with invalid/missing vendor */
  FALLBACK_ID: 'other',
} as const;
