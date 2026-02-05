/**
 * Badge configuration for desktop environments and kernel types
 */

export interface BadgeConfig {
  label: string;
  color: string;
}

/**
 * Desktop environment badges
 */
export const DESKTOP_BADGES: Record<string, BadgeConfig> = {
  'gnome': { label: 'GNOME', color: '#4a86cf' },
  'kde': { label: 'KDE', color: '#1d99f3' },
  'xfce': { label: 'XFCE', color: '#2284f2' },
  'cinnamon': { label: 'Cinnamon', color: '#dc682e' },
  'budgie': { label: 'Budgie', color: '#6a9fb5' },
  'mate': { label: 'MATE', color: '#9bda5a' },
  'lxde': { label: 'LXDE', color: '#a4a4a4' },
  'lxqt': { label: 'LXQt', color: '#0192d3' },
  'i3': { label: 'i3wm', color: '#1a8cff' },
  'sway': { label: 'Sway', color: '#68b0d8' },
};

/**
 * Kernel type badges
 */
export const KERNEL_BADGES: Record<string, BadgeConfig> = {
  'current': { label: 'Current', color: '#10b981' },  // Green
  'edge': { label: 'Edge', color: '#ef4444' },        // Red
  'legacy': { label: 'Legacy', color: '#6b7280' },    // Gray
  'vendor': { label: 'Vendor', color: '#8b5cf6' },    // Purple
};

/**
 * List of desktop environment keys for filtering
 */
export const DESKTOP_ENVIRONMENTS = Object.keys(DESKTOP_BADGES);

/**
 * Get desktop environment from variant string
 */
export function getDesktopEnv(variant: string): string | null {
  const v = variant.toLowerCase();
  for (const key of DESKTOP_ENVIRONMENTS) {
    if (v.includes(key)) return key;
  }
  return null;
}

/**
 * Get kernel type from branch string
 */
export function getKernelType(branch: string): string | null {
  const b = branch.toLowerCase();
  for (const key of Object.keys(KERNEL_BADGES)) {
    if (b.includes(key)) return key;
  }
  return null;
}

/**
 * Adjust hex color brightness for gradient effects
 * @param hex - Hex color (e.g., "#ef4444")
 * @param percent - Brightness adjustment (-100 to 100, negative darkens, positive lightens)
 * @returns Adjusted hex color
 */
export function adjustBrightness(hex: string, percent: number): string {
  // Remove hash if present
  const color = hex.replace('#', '');

  // Parse hex to RGB
  const num = parseInt(color, 16);
  const r = (num >> 16) & 0xFF;
  const g = (num >> 8) & 0xFF;
  const b = num & 0xFF;

  // Calculate adjustment amount
  const amt = Math.round(2.55 * percent);

  // Apply adjustment and clamp to 0-255 range
  const R = Math.max(0, Math.min(255, r + amt));
  const G = Math.max(0, Math.min(255, g + amt));
  const B = Math.max(0, Math.min(255, b + amt));

  // Convert back to hex
  return '#' + (0x1000000 + R * 0x10000 + G * 0x100 + B).toString(16).slice(1);
}
