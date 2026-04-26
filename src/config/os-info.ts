/**
 * OS/Distro information configuration
 */

import debianLogo from '../assets/os-logos/debian.svg';
import ubuntuLogo from '../assets/os-logos/ubuntu.png';
import homeassistantLogo from '../assets/os-logos/homeassistant.png';
import openmediavaultLogo from '../assets/os-logos/openmediavault.jpeg';

export interface OsInfoConfig {
  name: string;
  color: string;
  logo: string;
}

export interface AppInfoConfig {
  name: string;
  color: string;
  badgeColor: string;
  logo: string | null;
}

/**
 * OS/Distro release information with local logos
 */
export const OS_INFO: Record<string, OsInfoConfig> = {
  // Debian releases
  'bookworm': { name: 'Debian 12', color: 'transparent', logo: debianLogo },
  'bullseye': { name: 'Debian 11', color: 'transparent', logo: debianLogo },
  'trixie': { name: 'Debian 13', color: 'transparent', logo: debianLogo },
  'forky': { name: 'Debian 14', color: 'transparent', logo: debianLogo },
  'sid': { name: 'Debian Sid', color: 'transparent', logo: debianLogo },
  // Ubuntu releases
  'noble': { name: 'Ubuntu 24.04', color: 'transparent', logo: ubuntuLogo },
  'jammy': { name: 'Ubuntu 22.04', color: 'transparent', logo: ubuntuLogo },
  'resolute': { name: 'Ubuntu 25.10', color: 'transparent', logo: ubuntuLogo },
  'plucky': { name: 'Ubuntu 25.04', color: 'transparent', logo: ubuntuLogo },
  'oracular': { name: 'Ubuntu 24.10', color: 'transparent', logo: ubuntuLogo },
  'focal': { name: 'Ubuntu 20.04', color: 'transparent', logo: ubuntuLogo },
  'mantic': { name: 'Ubuntu 23.10', color: 'transparent', logo: ubuntuLogo },
  'lunar': { name: 'Ubuntu 23.04', color: 'transparent', logo: ubuntuLogo },
};

/**
 * Special applications with their own branding
 */
export const APP_INFO: Record<string, AppInfoConfig> = {
  'homeassistant': { name: 'Home Assistant', color: 'transparent', badgeColor: '#18bcf2', logo: homeassistantLogo },
  'openmediavault': { name: 'OpenMediaVault', color: 'transparent', badgeColor: '#5dacdf', logo: openmediavaultLogo },
  'omv': { name: 'OpenMediaVault', color: 'transparent', badgeColor: '#5dacdf', logo: openmediavaultLogo },
  'nextcloud': { name: 'Nextcloud', color: '#0082c9', badgeColor: '#0082c9', logo: null },
  'openwrt': { name: 'OpenWrt', color: '#00a3e0', badgeColor: '#00a3e0', logo: null },
  'pihole': { name: 'Pi-hole', color: '#96060c', badgeColor: '#96060c', logo: null },
  'kodi': { name: 'Kodi', color: '#17b2e7', badgeColor: '#17b2e7', logo: null },
};

/**
 * Get OS info from distro release name
 */
export function getOsInfo(distroRelease: string): OsInfoConfig | null {
  const release = distroRelease.toLowerCase();
  for (const [key, info] of Object.entries(OS_INFO)) {
    if (release.includes(key)) {
      return info;
    }
  }
  return null;
}

/**
 * Get App info from preinstalled application
 */
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
