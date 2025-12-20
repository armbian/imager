/**
 * Shared utility functions and constants
 */

/** Default color for icons without specific branding */
export const DEFAULT_COLOR = '#64748b';

/**
 * Format file size in human-readable format
 * @param bytes - Size in bytes
 * @param unknownText - Text to show when size is 0 or unknown
 * @param precision - Whether to show precise values for small sizes (default: false)
 * @returns Formatted size string (e.g., "1.5 GB", "256 MB")
 */
export function formatFileSize(
  bytes: number,
  unknownText: string = 'Unknown',
  precision: boolean = false
): string {
  if (bytes === 0) return unknownText;

  if (precision) {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    if (bytes < 1024 * 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  }

  const gb = bytes / (1024 * 1024 * 1024);
  if (gb >= 1) return `${gb.toFixed(1)} GB`;
  const mb = bytes / (1024 * 1024);
  return `${mb.toFixed(0)} MB`;
}

/**
 * Preload an image and return whether it loaded successfully
 * @param url - Image URL to preload
 * @returns Promise that resolves to true if loaded, false if failed
 */
export function preloadImage(url: string): Promise<boolean> {
  return new Promise((resolve) => {
    const img = new Image();
    img.onload = () => resolve(true);
    img.onerror = () => resolve(false);
    img.src = url;
  });
}

/**
 * Extract error message from unknown error type
 * @param error - Unknown error (Error, string, or other)
 * @param fallback - Fallback message if extraction fails
 * @returns Error message string
 */
export function getErrorMessage(error: unknown, fallback: string = 'An error occurred'): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return fallback;
}

/**
 * Sort comparator for boards: Platinum > Standard > Community > EOS > TVB > WIP > Others (alphabetically)
 */
export function compareBoardsBySupport<T extends {
  has_platinum_support: boolean;
  has_standard_support: boolean;
  has_community_support: boolean;
  has_eos_support: boolean;
  has_tvb_support: boolean;
  has_wip_support: boolean;
  name: string;
}>(a: T, b: T): number {
  if (a.has_platinum_support && !b.has_platinum_support) return -1;
  if (!a.has_platinum_support && b.has_platinum_support) return 1;
  if (a.has_standard_support && !b.has_standard_support) return -1;
  if (!a.has_standard_support && b.has_standard_support) return 1;
  if (a.has_community_support && !b.has_community_support) return -1;
  if (!a.has_community_support && b.has_community_support) return 1;
  if (a.has_eos_support && !b.has_eos_support) return -1;
  if (!a.has_eos_support && b.has_eos_support) return 1;
  if (a.has_tvb_support && !b.has_tvb_support) return -1;
  if (!a.has_tvb_support && b.has_tvb_support) return 1;
  if (a.has_wip_support && !b.has_wip_support) return -1;
  if (!a.has_wip_support && b.has_wip_support) return 1;
  return a.name.localeCompare(b.name);
}
