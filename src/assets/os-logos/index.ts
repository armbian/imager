// OS Logos
import debianLogo from './debian.svg';
import ubuntuLogo from './ubuntu.png';
import armbianLogo from '../armbian-logo.png';

// App Logos
import homeassistantLogo from './homeassistant.png';
import openmediavaultLogo from './openmediavault.jpeg';

// Import OS_INFO from config as single source of truth
import { OS_INFO } from '../../config/os-info';

export const osLogos: Record<string, string> = {
  debian: debianLogo,
  ubuntu: ubuntuLogo,
  armbian: armbianLogo,
};

export const appLogos: Record<string, string> = {
  homeassistant: homeassistantLogo,
  'home assistant': homeassistantLogo,
  openmediavault: openmediavaultLogo,
  omv: openmediavaultLogo,
};

/**
 * Get the appropriate logo for an image based on distro release and preinstalled app
 * Priority: preinstalled app logo > OS logo based on distro > null (for generic icon)
 * For custom images, also checks the filename for OS/app keywords
 * Returns null if no matching logo found (caller should show generic icon)
 */
export function getImageLogo(distroRelease: string, preinstalledApp?: string): string | null {
  // First check if there's a preinstalled app with a logo
  if (preinstalledApp) {
    const appKey = preinstalledApp.toLowerCase();
    for (const [key, logo] of Object.entries(appLogos)) {
      if (appKey.includes(key)) {
        return logo;
      }
    }
  }

  // Check OS based on distro release (also works for custom image filenames)
  const distro = distroRelease.toLowerCase();

  // Check for apps in filename (for custom images)
  for (const [key, logo] of Object.entries(appLogos)) {
    if (distro.includes(key)) {
      return logo;
    }
  }

  // Check OS_INFO for known codenames (single source of truth)
  for (const [codename, info] of Object.entries(OS_INFO)) {
    if (distro.includes(codename)) {
      return info.logo;
    }
  }

  // Check for explicit OS names
  if (distro.includes('ubuntu')) {
    return osLogos.ubuntu;
  }

  if (distro.includes('debian')) {
    return osLogos.debian;
  }

  if (distro.includes('armbian')) {
    return osLogos.armbian;
  }

  // Return null for unrecognized OS (caller should show generic icon)
  return null;
}

/**
 * Get the OS name from distro release
 */
export function getOsName(distroRelease: string): string {
  const distro = distroRelease.toLowerCase();

  // Check OS_INFO for known codenames
  for (const [codename, info] of Object.entries(OS_INFO)) {
    if (distro.includes(codename)) {
      return info.name;
    }
  }

  if (distro.includes('ubuntu')) {
    return 'Ubuntu';
  }

  if (distro.includes('debian')) {
    return 'Debian';
  }

  return 'Armbian';
}
