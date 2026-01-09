/**
 * Configuration exports
 */

// OS/App information
export {
  OS_INFO,
  APP_INFO,
  getOsInfo,
  getAppInfo,
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
  COLORS,
  QR_CODE,
  UI,
  VENDOR,
  type DeviceType,
} from './constants';

// Device colors
export { DEVICE_COLORS, getDeviceColors, type DeviceColorConfig } from './deviceColors';
