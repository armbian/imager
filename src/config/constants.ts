// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

/** Application constants and configuration values */

/** Polling intervals in milliseconds */
export const POLLING = {
  DEVICE_CHECK: 2000,
  DOWNLOAD_PROGRESS: 250,
  FLASH_PROGRESS: 250,
  CONNECTIVITY_CHECK: 30000,
} as const;

export type DeviceType = 'system' | 'sd' | 'usb' | 'sata' | 'sas' | 'nvme' | 'hdd';

export const LINKS = {
  GITHUB_REPO: 'https://github.com/armbian/imager',
  DOCS: 'https://docs.armbian.com',
  FORUM: 'https://forum.armbian.com',
  MOTD: 'https://raw.githubusercontent.com/armbian/os/main/motd.json',
} as const;

/** Timing constants in milliseconds */
export const TIMING = {
  MOTD_ROTATION: 30000,
  COPIED_NOTIFICATION: 2000,
  TOAST_DURATION: 3000,
  COVERFLOW_ADVANCE: 3600,
} as const;

export const CACHE = {
  /** Auto-delete a cached image after this many consecutive flash failures */
  MAX_FLASH_FAILURES: 3,
  /** Default maximum cache size: 20 GB */
  DEFAULT_SIZE: 20 * 1024 * 1024 * 1024,
  SIZE_OPTIONS: [
    { value: 5 * 1024 * 1024 * 1024, label: '5 GB' },
    { value: 10 * 1024 * 1024 * 1024, label: '10 GB' },
    { value: 20 * 1024 * 1024 * 1024, label: '20 GB' },
    { value: 50 * 1024 * 1024 * 1024, label: '50 GB' },
    { value: 100 * 1024 * 1024 * 1024, label: '100 GB' },
  ],
  EMPTY_BREAKDOWN: { images: 0, assets: 0, total: 0 },
} as const;

/** Custom DOM events for inter-component communication */
export const EVENTS = {
  MOTD_CHANGED: 'armbian-motd-changed',
  SETTINGS_CHANGED: 'armbian-settings-changed',
  CACHE_IMAGE_REUSE: 'armbian-cache-image-reuse',
  PROFILES_CHANGED: 'armbian-autoconfig-profiles-changed',
  AUTOCONFIG_PROFILE_CREATED: 'armbian-autoconfig-profile-created',
  OPEN_SETTINGS: 'armbian-open-settings',
} as const;

/** Storage key prefixes for sessionStorage/localStorage */
export const STORAGE_KEYS = {
  /** Prefix, appended with the image URL */
  FLASH_FAILURE_PREFIX: 'flash_failure_count_',
} as const;

/** Settings store configuration */
export const SETTINGS = {
  FILE: 'settings.json',
  KEYS: {
    THEME: 'theme',
    LANGUAGE: 'language',
    SHOW_MOTD: 'show_motd',
    SHOW_WELCOME: 'show_welcome',
    SHOW_UPDATER_MODAL: 'show_updater_modal',
    DEVELOPER_MODE: 'developer_mode',
    SKIP_VERIFY: 'skip_verify',
    FORCE_OFFLINE: 'force_offline',
    CACHE_ENABLED: 'cache_enabled',
    CACHE_MAX_SIZE: 'cache_max_size',
    ARMBIAN_BOARD_DETECTION: 'armbian_board_detection',
    AUTOCONFIG_PROFILES: 'autoconfig_profiles',
    ALLOW_SYSTEM_DEVICES: 'allow_system_devices',
    REDUCED_MOTION: 'reduced_motion',
  },
  DEFAULTS: {
    THEME: 'auto',
    LANGUAGE: 'en',
    SHOW_MOTD: true,
    SHOW_WELCOME: true,
    SHOW_UPDATER_MODAL: true,
    DEVELOPER_MODE: false,
    SKIP_VERIFY: false,
    FORCE_OFFLINE: false,
    CACHE_ENABLED: true,
    ARMBIAN_BOARD_DETECTION: 'modal',
    AUTOCONFIG_PROFILES: [] as [],
    ALLOW_SYSTEM_DEVICES: false,
    REDUCED_MOTION: 'auto',
  },
  /** Motion preference: follow the OS, or force animations on/off regardless. */
  MOTION_MODES: {
    AUTO: 'auto',
    REDUCE: 'reduce',
    FULL: 'full',
  },
  ARMBIAN_DETECTION_MODES: {
    DISABLED: 'disabled',
    MODAL: 'modal',
    AUTO: 'auto',
  },
} as const;

/** Shared color palette referenced across config modules */
export const PALETTE = {
  RED: '#ef4444',
  GREEN: '#10b981',
  AMBER: '#f59e0b',
  BLUE: '#3b82f6',
  VIOLET: '#8b5cf6',
  CYAN: '#06b6d4',
  SKY: '#0ea5e9',
  SLATE: '#64748b',
} as const;

/** UI color constants */
export const COLORS = {
  DEFAULT_ICON: PALETTE.SLATE,
  QR_DARK: '#000000',
  QR_LIGHT: '#ffffff',
} as const;

export const QR_CODE = {
  WIDTH: 120,
    MARGIN: 1,
} as const;

/** UI dimension constants */
export const UI = {
  /** Staggered animation timing for list/grid items */
  STAGGER: {
    MAX_INDEX: 18,
    STEP_S: 0.04,
  },
  /** Modal exit animation duration in milliseconds; mirrors --dur-base */
  MODAL_EXIT_MS: 200,
  /** Toast exit animation duration; mirrors --dur-medium */
  TOAST_EXIT_MS: 300,
  /** Must outlast the last .split.is-entering .side-step delay plus --dur-slow in styles/layout.css */
  ENTRANCE_MS: 1100,
  CACHE_ROW_STAGGER_MS: 25,
  /** Welcome coverflow: per-distance values are indexed by distance from the focused slide */
  COVERFLOW: {
    FLANK: 2,
    POOL_SIZE: 24,
    SCALE: [1, 0.72, 0.5],
    OPACITY: [1, 0.55, 0.22],
    BLUR_PX: [0, 1.5, 3],
    Z_BASE: 10,
  },
  SKELETON: {
    MIN_VISIBLE_MS: 300,
    MANUFACTURER_PANEL: 12,
    BOARD_PANEL: 10,
    OS_PANEL: 8,
    DEVICE_PANEL: 4,
  },
  MARQUEE: {
    MEASURE_DELAY_MS: 50,
    SEPARATOR_WIDTH: 5,
    SUMMARY_VALUE_WIDTH: 340,
    SUMMARY_TARGET_WIDTH: 300,
  },
  /** Paged card grid fit; COL_MIN, GAP and PAD mirror .mfr-grid in styles/layout.css */
  GRID: {
    COL_MIN: 220,
    GAP: 18,
    PAD: 24,
    /** Always show at least this many full rows so a short window isn't left with a lonely row */
    MIN_ROWS: 2,
    /** Default page cap, sized to avoid loading too many heavy cards (board photos) at once */
    MAX_PER_PAGE: 40,
    INITIAL_PAGE_SIZE: 15,
    /** Logos are light and preloaded, so a high cap fills large monitors without splitting a screenful */
    MANUFACTURER_MAX_PER_PAGE: 120,
    /** Card min-height plus row gap: .mfr-card 206 + 18, .board-card 250 + 18 */
    CARD_ROW: {
      MANUFACTURER: 224,
      BOARD: 268,
    },
  },
  OS_REC_MAX_COLUMNS: 3,
  /** Percent floor that keeps a non-empty storage bar segment visible */
  STORAGE_BAR_MIN_PERCENT: 4,
  /** Icon sizes in pixels */
  ICON_SIZE: {
    SEARCH: 18,
    FLASH_STAGE: 32,
    EMPTY_STATE: 30,
    EMPTY_STATE_ACTION: 15,
  },
} as const;

/** Vendor/manufacturer constants */
export const VENDOR = {
  /** Fallback vendor ID for boards with invalid/missing vendor */
  FALLBACK_ID: 'other',
} as const;

/** Special board slugs for synthetic selection entries */
export const SLUGS = {
  CUSTOM: 'custom',
  CACHED: 'cached',
  DETECTED: 'detected',
} as const;

/** Image variant identifiers for non-standard image sources */
export const IMAGE_VARIANT = {
  CACHED: 'cached',
  CUSTOM: 'custom',
} as const;
