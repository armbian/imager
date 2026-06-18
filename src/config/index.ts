// Configuration exports

// OS/App information
export {
  OS_INFO,
  APP_INFO,
  getOsInfo,
  getAppInfo,
  getImageVariantLabel,
  type OsInfoConfig,
  type AppInfoConfig,
} from './os-info';

// Badge configuration
export {
  DESKTOP_BADGES,
  KERNEL_BADGES,
  DESKTOP_ENVIRONMENTS,
  getDesktopEnv,
  getKernelType,
  adjustBrightness,
  type BadgeConfig,
} from './badges';

// Constants and polling intervals
export {
  POLLING,
  LINKS,
  TIMING,
  CACHE,
  EVENTS,
  STORAGE_KEYS,
  SETTINGS,
  PALETTE,
  COLORS,
  QR_CODE,
  UI,
  VENDOR,
  SLUGS,
  IMAGE_VARIANT,
  type DeviceType,
} from './constants';

// Support tiers
export {
  SUPPORT_TIER,
  SUPPORT_TIER_LABEL,
  SUPPORT_TIER_ORDER,
  PARTNER_TIER_RANK,
} from './supportTiers';

// Image filters
export { isTrunkImage, IMAGE_FILTER_PREDICATES, FILTER_BUTTONS, categoryOf } from './imageFilters';
export type { OsCategory } from './imageFilters';
export { qdlInstructionsKey } from './qdlBoards';
