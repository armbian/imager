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

const GITHUB = 'https://github.com';
const GITHUB_REPO = `${GITHUB}/armbian/imager`;

export const LINKS = {
  GITHUB,
  GITHUB_REPO,
  ISSUES: `${GITHUB_REPO}/issues`,
  DOCS: 'https://docs.armbian.com',
  FORUM: 'https://forum.armbian.com',
  MOTD: 'https://raw.githubusercontent.com/armbian/os/main/motd.json',
  WEBSITE: 'https://www.armbian.com',
  DONATE: 'https://www.armbian.com/donate',
  /** Shown in the log upload description; the upload endpoint itself lives in the backend config */
  PASTE_HOST: 'paste.armbian.com',
} as const;

/** Timing constants in milliseconds */
export const TIMING = {
  MOTD_ROTATION: 30000,
  COPIED_NOTIFICATION: 2000,
  TOAST_DURATION: 3000,
  COVERFLOW_ADVANCE: 3600,
} as const;

export const BYTES_PER_KB = 1024;
export const BYTES_PER_MB = BYTES_PER_KB * BYTES_PER_KB;

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
  /** Carries the opt-in autoconfig profile id (or null) picked at flash time */
  AUTOCONFIG_PROFILE_SELECTED: 'armbian-autoconfig-profile-selected',
  CONNECTIVITY_RECHECK: 'armbian-connectivity-recheck',
} as const;

export const SETTINGS_VIEW = {
  GENERAL: 'general',
  WRITING: 'writing',
  PROFILES: 'profiles',
  DOWNLOADS: 'downloads',
  ABOUT: 'about',
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
  THEME_MODES: {
    LIGHT: 'light',
    DARK: 'dark',
    AUTO: 'auto',
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

/** Must match the platform ids returned by the backend get_system_info */
export const PLATFORM = {
  LINUX: 'linux',
  MACOS: 'macos',
  WINDOWS: 'windows',
} as const;

export const PLATFORM_UA = {
  MACOS: 'Mac',
  WINDOWS: 'Windows',
} as const;

export const PLATFORM_CLASS = {
  MACOS: 'is-macos',
} as const;

export const PLATFORM_LABEL: Record<string, string> = {
  [PLATFORM.MACOS]: 'macOS',
  [PLATFORM.WINDOWS]: 'Windows',
  [PLATFORM.LINUX]: 'Linux',
};

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
  TEAL: '#14b8a6',
  GRAY: '#6b7280',
  ORANGE: '#f2651f',
} as const;

/** UI color constants */
export const COLORS = {
  DEFAULT_ICON: PALETTE.SLATE,
  QR_DARK: '#000000',
  QR_LIGHT: '#ffffff',
  ON_TILE: '#ffffff',
  TERMINAL_FG: '#FFF',
  TERMINAL_BG: '#000',
} as const;

export const QR_CODE = {
  WIDTH: 120,
    MARGIN: 1,
} as const;

/** UI dimension constants */
export const UI = {
  /** Boards row grid in px (GAP twins .boards-row__list); REMOVE_MS mirrors --pager-swap-out in styles/theme.css */
  BOARDS_ROW: { MIN_CARD_WIDTH: 170, GAP: 12, MIN_SLOTS: 2, DEFAULT_SLOTS: 4, REMOVE_MS: 100 },
  /** Joins the parts of a one-line summary (nav sub-lines, profile facts) */
  SUMMARY_SEPARATOR: ' · ',
  /** Joins list items where the WebView has no Intl.ListFormat */
  LIST_SEPARATOR: ', ',
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
  /** Paged card grid fit; COL_MIN and GAP mirror .mfr-grid in styles/layout.css */
  GRID: {
    COL_MIN: 220,
    GAP: 18,
    MIN_ROWS: 1,
    /** Default page cap, sized to avoid loading too many heavy cards (board photos) at once */
    MAX_PER_PAGE: 40,
    INITIAL_PAGE_SIZE: 15,
    /** Logos are light and preloaded, so a high cap fills large monitors without splitting a screenful */
    MANUFACTURER_MAX_PER_PAGE: 120,
    /** Card row (resting height plus gap) for a grid without --grid-card-min/max; mirrors theme.css */
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
    PROFILE_GLYPH: 11,
    WIZARD_ITEM: 14,
    WIZARD_TILE: 22,
  },
  /** Minimum margin between an anchored popover and the window edge */
  POPOVER_EDGE: 12,
  PROFILE_MENU: {
    WIDTH: 320,
    GAP: 8,
  },
  NEW_PROFILE_MENU: {
    WIDTH: 300,
  },
  SELECT_MENU: {
    MIN_WIDTH: 320,
    GAP: 6,
  },
  /** Depth zoom-through page transition; twins of the --settings-* custom properties in settings-page.css */
  SETTINGS_PAGE: {
    EXIT_MS: 200,
    ENTER_DELAY_MS: 200,
    ENTER_MS: 390,
    HEADER_OUT_MS: 140,
    HEADER_IN_MS: 260,
    RING_DELAY_MS: 200,
  },
  /** Share of the hero that must be visible before its entrance plays */
  SETTINGS_HERO: {
    PLAY_THRESHOLD: 0.4,
  },
  /** Profile list grid; CARD_H and GAP mirror .profile-card and .profiles-list in styles/profiles-list.css */
  PROFILES: {
    /** Narrowest card that keeps the square tile and two readable fact columns; two columns from twice this */
    MIN_CARD_WIDTH: 520,
    /** From this card width the four areas sit in labelled columns, wide enough for a fixed address */
    WIDE_CARD_MIN: 900,
    /** Two rows fit under the hero on a 700px window */
    CARD_H: 140,
    GAP: 14,
    MIN_ROWS: 1,
    MAX_PER_PAGE: 24,
    /** Network, user, language and time, access: the facts a card's spec strip lists */
    SPEC_AREAS: 4,
    CAROUSEL_INTERVAL_MS: 2400,
    CAROUSEL_SLIDE_MS: 320,
    /** Share of the square tile a board photo's visible content spans */
    TILE_FILL: 0.84,
  },
  /** Fitting a photo by its opaque pixels (utils/imageFit.ts) */
  IMAGE_FIT: {
    /** Alpha above which a pixel counts as content; drop shadows fade below it */
    ALPHA_MIN: 16,
    /** Upscale cap, so a tiny or nearly empty photo does not blow up */
    MAX_SCALE: 2.4,
    /** Longest side the pixels are sampled at; plenty for a bounding box and cheap for a menu of logos */
    SAMPLE_MAX: 160,
    /** Mean luminance above which a logo's ink counts as light and is darkened for the light plate */
    LIGHT_INK: 0.8,
    /** Ink covering more of its own box than this is an opaque plate, not light lettering */
    PLATE_COVERAGE: 0.9,
    /** Share of ink that already reads on white (dark or saturated) above which a light logo keeps the light plate */
    LIGHT_VISIBLE_MAX: 0.05,
    /** Luminance and chroma that make a pixel read on white */
    VISIBLE_LUM: 0.7,
    VISIBLE_CHROMA: 0.4,
  },
  /** Editor action bar slide-down; mirrors --action-bar-out in styles/theme.css */
  ACTION_BAR: {
    OUT_MS: 160,
  },
  /** Marks focus moved by code after a mouse action, so base.css hides the keyboard ring there */
  POINTER_FOCUS_ATTR: 'data-pointer-focus',
  /** Choose boards sheet: page sizes, list caps, and timings twinned with styles/theme.css */
  BOARD_SHEET: {
    PER_PAGE: 8,
    REVIEW_PER_PAGE: 6,
    RECENT_BRANDS: 4,
    /** Photos in the footer stack, the last one turning into "+N" when more are picked */
    LEDGER_THUMBS: 4,
    REVIEW_VENDORS: 4,
    PEEK_PHOTOS: 3,
    BRAND_HITS: 5,
    UNDO_MS: 4200,
    /** Share of the photo well a board's visible content spans */
    PHOTO_FILL: 0.72,
    /** Share of a logo tile's inner area a vendor mark covers, so wordmarks and square marks weigh alike */
    LOGO_AREA: 0.5,
    /** Mirrors --brand-menu-close in styles/theme.css */
    MENU_CLOSE_MS: 100,
    SEARCH_KEY: '/',
  },
  /** Soft tag fill and ring alphas on a cached image's colour chip (CacheManagerModal) */
  TAG_ALPHA: { SOFT: 0.14, RING: 0.36 },
  /** Page change animation (components/shared/PageSwap.tsx); outlasts --page-swap-dur in case animationend never fires */
  PAGE_SWAP: {
    OUT_FALLBACK_MS: 400,
  },
} as const;

/** Autoconfig limits and timings for the profile forms */
export const AUTOCONFIG = {
  /** WPA passphrase length; a 64-character hex PSK also works, so the check never blocks */
  WIFI_KEY_MIN: 8,
  WIFI_KEY_MAX: 63,
  KEY_LOOKUP_DEBOUNCE_MS: 600,
} as const;

/** Vendor/manufacturer constants */
export const VENDOR = {
  /** Fallback vendor ID for boards with invalid/missing vendor */
  FALLBACK_ID: 'other',
  FALLBACK_NAME: 'Other',
  /** Vendor shown for a cached image whose board came from its filename only */
  UNKNOWN_NAME: 'Unknown',
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

export const LOCAL_SOURCE_LABEL = {
  [IMAGE_VARIANT.CACHED]: 'Cached',
  [IMAGE_VARIANT.CUSTOM]: 'Custom',
} as const;
