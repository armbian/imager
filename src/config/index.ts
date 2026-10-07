// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

// Configuration exports

// OS/App information
export {
  OS_INFO,
  APP_INFO,
  getOsInfo,
  getOsName,
  getAppInfo,
  getImageVariantLabel,
  type OsInfoConfig,
  type AppInfoConfig,
} from './os-info';

// Badge configuration
export {
  DESKTOP_BADGES,
  VARIANT_BADGES,
  KERNEL_BADGES,
  STORAGE_BADGES,
  CLI_BADGE,
  DESKTOP_ENVIRONMENTS,
  getDesktopEnv,
  getVariantBadge,
  getKernelType,
  type BadgeConfig,
} from './badges';

// Constants and polling intervals
export {
  POLLING,
  LINKS,
  TIMING,
  BYTES_PER_KB,
  BYTES_PER_MB,
  CACHE,
  EVENTS,
  STORAGE_KEYS,
  SETTINGS,
  PALETTE,
  COLORS,
  QR_CODE,
  UI,
  AUTOCONFIG,
  VENDOR,
  SLUGS,
  IMAGE_VARIANT,
  LOCAL_SOURCE_LABEL,
  PLATFORM,
  PLATFORM_UA,
  PLATFORM_CLASS,
  PLATFORM_LABEL,
  type DeviceType,
} from './constants';

// Support tiers
export {
  SUPPORT_TIER,
  SUPPORT_TIER_LABEL,
  SUPPORT_TIER_ORDER,
  PARTNER_TIER_RANK,
  PARTNER_TIER_UNRANKED,
} from './supportTiers';

// Image filters
export {
  isTrunkImage,
  IMAGE_FILTER_PREDICATES,
  FILTER_BUTTONS,
  OS_CATEGORY_GROUPS,
  IMAGE_STATUS_COLOR,
  PROMOTED_COLOR,
  categoryOf,
} from './imageFilters';
export type { OsCategory } from './imageFilters';
export { qdlInstructionsKey } from './qdlBoards';
