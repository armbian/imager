/** Badge configuration for desktop environments and kernel types */

import { PALETTE } from './constants';

export interface BadgeConfig {
  label: string;
  color: string;
}

/** Desktop environment badges */
export const DESKTOP_BADGES: Record<string, BadgeConfig> = {
  'gnome': { label: 'GNOME', color: '#4a86cf' },
  'kde': { label: 'KDE', color: '#1d99f3' },
  'xfce': { label: 'XFCE', color: '#2284f2' },
  'cinnamon': { label: 'Cinnamon', color: '#dc682e' },
  'budgie': { label: 'Budgie', color: '#6a9fb5' },
  'mate': { label: 'MATE', color: '#9bda5a' },
  'lxde': { label: 'LXDE', color: '#a4a4a4' },
  'lxqt': { label: 'LXQt', color: '#0192d3' },
  'i3': { label: 'i3WM', color: '#1a8cff' },
  'sway': { label: 'Sway', color: '#68b0d8' },
};

/** Kernel type badges */
export const KERNEL_BADGES: Record<string, BadgeConfig> = {
  'current': { label: 'Current', color: PALETTE.GREEN },
  'edge': { label: 'Edge', color: PALETTE.RED },
  'legacy': { label: 'Legacy', color: '#6b7280' },
  'vendor': { label: 'Vendor', color: PALETTE.VIOLET },
  'collabora': { label: 'Collabora', color: PALETTE.AMBER },
  'sc8280xp': { label: 'SC8280XP', color: PALETTE.CYAN },
  'cloud': { label: 'Cloud', color: PALETTE.SKY },
};

/** Desktop environment keys, used for filtering */
export const DESKTOP_ENVIRONMENTS = Object.keys(DESKTOP_BADGES);

/** Get the desktop environment from a variant string */
export function getDesktopEnv(variant: string): string | null {
  const v = variant.toLowerCase();
  for (const key of DESKTOP_ENVIRONMENTS) {
    if (v.includes(key)) return key;
  }
  return null;
}

/** Get the kernel type from a branch string */
export function getKernelType(branch: string): string | null {
  const b = branch.toLowerCase();
  for (const key of Object.keys(KERNEL_BADGES)) {
    if (b.includes(key)) return key;
  }
  return null;
}

/** Adjust a hex color's brightness by `percent` (-100 darkens to +100 lightens) */
export function adjustBrightness(hex: string, percent: number): string {
  const color = hex.replace('#', '');

  const num = parseInt(color, 16);
  const r = (num >> 16) & 0xFF;
  const g = (num >> 8) & 0xFF;
  const b = num & 0xFF;

  // percent maps to a -255..+255 offset applied to each channel
  const amt = Math.round(2.55 * percent);
  const R = Math.max(0, Math.min(255, r + amt));
  const G = Math.max(0, Math.min(255, g + amt));
  const B = Math.max(0, Math.min(255, b + amt));

  return '#' + (0x1000000 + R * 0x10000 + G * 0x100 + B).toString(16).slice(1);
}
