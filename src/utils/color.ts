// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

import type { CSSProperties } from 'react';

/** Parse a hex color (#rgb or #rrggbb) into its red/green/blue components. */
export function hexToRgb(hex: string): { r: number; g: number; b: number } {
  let value = hex.replace('#', '');
  // Expand shorthand form (e.g. "0af" -> "00aaff")
  if (value.length === 3) {
    value = value
      .split('')
      .map((c) => c + c)
      .join('');
  }
  const num = parseInt(value, 16);
  return {
    r: (num >> 16) & 255,
    g: (num >> 8) & 255,
    b: num & 255,
  };
}

/** Build an rgba() string from a hex color and an alpha value. */
export function hexToRgba(hex: string, alpha: number): string {
  const { r, g, b } = hexToRgb(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
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

/** CSS variables for a solid gradient badge (.badge-solid) tinted by `hex`. */
export function solidBadgeVars(hex: string): CSSProperties {
  return {
    '--badge-from': hex,
    '--badge-to': adjustBrightness(hex, -20),
    '--badge-glow': hexToRgba(hex, 0.4),
  } as CSSProperties;
}
