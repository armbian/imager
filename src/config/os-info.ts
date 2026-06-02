/** OS/Distro information configuration */

import type { ImageInfo } from '../types';
import { getDesktopEnv, DESKTOP_BADGES } from './badges';

export interface OsInfoConfig {
  name: string;
  color: string;
}

export interface AppInfoConfig {
  name: string;
  /** Optional short badge label (e.g. "SDK"); falls back to `name` when omitted */
  badge?: string;
  color: string;
  badgeColor: string;
}

/** OS/Distro release information */
export const OS_INFO: Record<string, OsInfoConfig> = {
  // Debian releases
  'bookworm': { name: 'Debian 12', color: 'transparent' },
  'bullseye': { name: 'Debian 11', color: 'transparent' },
  'trixie': { name: 'Debian 13', color: 'transparent' },
  'forky': { name: 'Debian 14', color: 'transparent' },
  'sid': { name: 'Debian Sid', color: 'transparent' },
  // Ubuntu releases
  'noble': { name: 'Ubuntu 24.04', color: 'transparent' },
  'jammy': { name: 'Ubuntu 22.04', color: 'transparent' },
  'resolute': { name: 'Ubuntu 26.04', color: 'transparent' },
  'plucky': { name: 'Ubuntu 25.04', color: 'transparent' },
  'oracular': { name: 'Ubuntu 24.10', color: 'transparent' },
  'focal': { name: 'Ubuntu 20.04', color: 'transparent' },
  'mantic': { name: 'Ubuntu 23.10', color: 'transparent' },
  'lunar': { name: 'Ubuntu 23.04', color: 'transparent' },
};

/** Special applications with their own branding */
export const APP_INFO: Record<string, AppInfoConfig> = {
  'homeassistant': { name: 'Home Assistant', color: 'transparent', badgeColor: '#18bcf2' },
  'openmediavault': { name: 'OpenMediaVault', color: 'transparent', badgeColor: '#5dacdf' },
  'omv': { name: 'OpenMediaVault', color: 'transparent', badgeColor: '#5dacdf' },
  'sdk': { name: 'Code server + Armbian sources', badge: 'SDK', color: 'transparent', badgeColor: '#1e88e5' },
  'openhab': { name: 'openHAB', color: 'transparent', badgeColor: '#e64a19' },
  'kali': { name: 'Kali Linux', color: 'transparent', badgeColor: '#367bf0' },
};

/** Get OS info from a distro release name */
export function getOsInfo(distroRelease: string): OsInfoConfig | null {
  const release = distroRelease.toLowerCase();
  for (const [key, info] of Object.entries(OS_INFO)) {
    if (release.includes(key)) {
      return info;
    }
  }
  return null;
}

/** Get app info from a preinstalled application name */
export function getAppInfo(app: string | null): AppInfoConfig | null {
  if (!app) return null;
  const appLower = app.toLowerCase();
  for (const [key, info] of Object.entries(APP_INFO)) {
    if (appLower.includes(key)) {
      return info;
    }
  }
  return null;
}

/** Short variant label for an image: app name, else desktop badge label, else minimal fallback. */
export function getImageVariantLabel(image: ImageInfo, t: (key: string) => string): string {
  const appInfo = getAppInfo(image.preinstalled_application);
  if (appInfo) return appInfo.badge ?? appInfo.name;

  const desktopEnv = getDesktopEnv(image.image_variant);
  if (desktopEnv && DESKTOP_BADGES[desktopEnv]) return DESKTOP_BADGES[desktopEnv].label;

  return t('modal.minimal');
}
