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
