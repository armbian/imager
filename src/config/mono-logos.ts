/**
 * Monochrome (single-path, transparent-cutout) logo set used across the app for OS/app marks:
 * the OsPanel split cards force them to a white silhouette over the distro gradient, while the
 * flash badge and cache manager tint them to the foreground colour via a CSS mask. This is the
 * single icon source; the old colour raster logos were removed.
 */

import ubuntuMono from '../assets/os-logos/mono/ubuntu.svg';
import debianMono from '../assets/os-logos/mono/debian.svg';
import homeassistantMono from '../assets/os-logos/mono/homeassistant.svg';
import kaliMono from '../assets/os-logos/mono/kali.svg';
import openhabMono from '../assets/os-logos/mono/openhab.svg';
import openmediavaultMono from '../assets/os-logos/mono/openmediavault.svg';
import vscodeMono from '../assets/os-logos/mono/vscode.svg';

/** Preinstalled-application marks, matched against the application substring. */
const APP_MONO: Record<string, string> = {
  // The Armbian SDK ships code-server, so it uses the VS Code mark.
  sdk: vscodeMono,
  homeassistant: homeassistantMono,
  openmediavault: openmediavaultMono,
  omv: openmediavaultMono,
  kali: kaliMono,
  openhab: openhabMono,
};

/**
 * Resolve a monochrome mark for an image, by app first then distro.
 * Returns null when no clean vector mark exists (caller shows a generic icon).
 */
export function getMonoLogo(distroRelease: string, app?: string | null): string | null {
  if (app) {
    const appKey = app.toLowerCase();
    for (const [key, logo] of Object.entries(APP_MONO)) {
      if (appKey.includes(key)) return logo;
    }
  }

  const distro = distroRelease.toLowerCase();
  if (distro.includes('ubuntu')) return ubuntuMono;
  if (distro.includes('debian')) return debianMono;

  // Ubuntu/Debian codenames used by the Armbian API.
  if (/(noble|jammy|resolute|plucky|oracular|focal|mantic|lunar)/.test(distro)) return ubuntuMono;
  if (/(bookworm|bullseye|trixie|forky|sid)/.test(distro)) return debianMono;

  return null;
}
