// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com
// Copyright (c) 2026 Igor Pecovnik, igor@armbian.com

/** OS/Distro information configuration */

import type { ImageInfo } from '../types';
import { getVariantBadge } from './badges';

/** Distro families; values double as the family names matched in a distro string */
export const OS_FAMILY = {
  DEBIAN: 'debian',
  UBUNTU: 'ubuntu',
} as const;

export type OsFamily = (typeof OS_FAMILY)[keyof typeof OS_FAMILY];

const OS_FAMILY_NAME: Record<OsFamily, string> = {
  [OS_FAMILY.DEBIAN]: 'Debian',
  [OS_FAMILY.UBUNTU]: 'Ubuntu',
};

/** OS name shown when a distro string matches no known release or family */
const FALLBACK_OS_NAME = 'Armbian';

export interface OsInfoConfig {
  name: string;
  color: string;
  family: OsFamily;
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
  'bookworm': { name: 'Debian 12', color: 'transparent', family: OS_FAMILY.DEBIAN },
  'bullseye': { name: 'Debian 11', color: 'transparent', family: OS_FAMILY.DEBIAN },
  'trixie': { name: 'Debian 13', color: 'transparent', family: OS_FAMILY.DEBIAN },
  'forky': { name: 'Debian 14', color: 'transparent', family: OS_FAMILY.DEBIAN },
  'sid': { name: 'Debian Sid', color: 'transparent', family: OS_FAMILY.DEBIAN },
  // Ubuntu releases
  'noble': { name: 'Ubuntu 24.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'jammy': { name: 'Ubuntu 22.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'resolute': { name: 'Ubuntu 26.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'plucky': { name: 'Ubuntu 25.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'oracular': { name: 'Ubuntu 24.10', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'focal': { name: 'Ubuntu 20.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'mantic': { name: 'Ubuntu 23.10', color: 'transparent', family: OS_FAMILY.UBUNTU },
  'lunar': { name: 'Ubuntu 23.04', color: 'transparent', family: OS_FAMILY.UBUNTU },
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

/** Human-readable OS name for a distro release: the known release, else its family, else Armbian. */
export function getOsName(distroRelease: string): string {
  const info = getOsInfo(distroRelease);
  if (info) return info.name;
  const distro = distroRelease.toLowerCase();
  if (distro.includes(OS_FAMILY.UBUNTU)) return OS_FAMILY_NAME[OS_FAMILY.UBUNTU];
  if (distro.includes(OS_FAMILY.DEBIAN)) return OS_FAMILY_NAME[OS_FAMILY.DEBIAN];
  return FALLBACK_OS_NAME;
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

  return getVariantBadge(image.image_variant)?.label ?? t('modal.minimal');
}
