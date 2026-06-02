/** Distro-based theming helpers: gradients, accent colour and per-card CSS variables. */

import type { CSSProperties } from 'react';
import { hexToRgba } from './index';

/** Distro-themed gradient for a recommended download card (mirrors Armbian-site). */
export function distroGradient(osName: string): string {
  const n = osName.toLowerCase();
  if (n.includes('ubuntu')) return 'linear-gradient(160deg, #f97b4b 0%, #e95420 42%, #9b3a8d 100%)';
  if (n.includes('debian')) return 'linear-gradient(160deg, #d63060 0%, #a80030 42%, #8b2f6b 100%)';
  return 'linear-gradient(160deg, #f9853f 0%, #e9601f 45%, #b23b1f 100%)';
}

/** Clean two-stop distro gradient for the split card's side block (no purple tail). */
export function distroBlock(osName: string): string {
  const n = osName.toLowerCase();
  if (n.includes('ubuntu')) return 'linear-gradient(135deg, #f0703a 0%, #d4400f 100%)';
  if (n.includes('debian')) return 'linear-gradient(135deg, #c43a63 0%, #9b0a30 100%)';
  return 'linear-gradient(135deg, #ef7836 0%, #d8541b 100%)';
}

/** Solid brand accent for a distro, used to tint the compact cards. */
export function distroAccent(osName: string): string {
  const n = osName.toLowerCase();
  if (n.includes('ubuntu')) return '#e95420';
  if (n.includes('debian')) return '#d70a53';
  return '#e9601f';
}

/** Build the per-card CSS variables that tint a compact card by its distro. */
export function distroVars(osName: string): CSSProperties {
  const hex = distroAccent(osName);
  return {
    '--distro': hex,
    '--distro-soft': hexToRgba(hex, 0.14),
    '--distro-ring': hexToRgba(hex, 0.42),
    '--distro-glow': hexToRgba(hex, 0.28),
  } as unknown as CSSProperties;
}
