import { COLORS, UI, SLUGS, SUPPORT_TIER_ORDER } from '../config';
import { getImageVariantLabel, getOsInfo } from '../config/os-info';
import { getDesktopEnv, DESKTOP_BADGES } from '../config/badges';
import type { ImageInfo } from '../types';

// Re-export color helpers from the dedicated color module
export { hexToRgb, hexToRgba } from './color';

/** Default color for icons without specific branding */
export const DEFAULT_COLOR = COLORS.DEFAULT_ICON;

/** Compute a staggered CSS animation delay clamped to UI.STAGGER.MAX_INDEX. */
export function staggerDelay(index: number): string {
  return `${Math.min(index, UI.STAGGER.MAX_INDEX) * UI.STAGGER.STEP_S}s`;
}

/** Format a Unix timestamp as relative time (e.g. "2 hours ago"). */
export function formatRelativeTime(
  timestamp: number,
  t: (key: string, opts?: Record<string, unknown>) => string
): string {
  const now = Math.floor(Date.now() / 1000);
  const diff = now - timestamp;

  if (diff < 60) return t('settings.cache.justNow');
  if (diff < 3600) return t('settings.cache.minutesAgo', { count: Math.floor(diff / 60) });
  if (diff < 86400) return t('settings.cache.hoursAgo', { count: Math.floor(diff / 3600) });
  return t('settings.cache.daysAgo', { count: Math.floor(diff / 86400) });
}

/** Fisher-Yates shuffle returning a new array. */
export function shuffle<T>(arr: T[]): T[] {
  const out = [...arr];
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

/** Whether a board is a real detected board (not the custom or cached placeholder). */
export function isDetectedBoard(board: { slug: string } | null | undefined): boolean {
  return !!board && board.slug !== SLUGS.CUSTOM && board.slug !== SLUGS.CACHED;
}

/** Format a byte size as human-readable text (e.g. "1.5 GB"); `unknownText` covers 0/unknown */
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

/** Format bytes as a human-readable string (e.g. "2.3 GB") */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
}

/** Parsed metadata from an Armbian image filename */
export interface ArmbianFilenameInfo {
  /** Board slug (lowercase, e.g. "nanopi-m5") */
  boardSlug: string;
  /** Version string (e.g. "25.02.0" or "26.2.0-trunk.493") */
  version: string | null;
  /** Distribution (e.g. "bookworm", "trixie") */
  distro: string | null;
  /** Branch (e.g. "current", "edge") */
  branch: string | null;
  /** Kernel version (e.g. "6.12.8") */
  kernel: string | null;
  /** Desktop environment or "minimal" */
  desktop: string | null;
}

/** Compression extensions an Armbian image can carry. */
export const COMPRESSION_EXTS = ['.xz', '.gz', '.zst', '.bz2'] as const;

/** True when a filename or URL ends with a known compression extension (i.e. a decompress step runs). */
export function isCompressedImage(nameOrUrl: string): boolean {
  const lower = nameOrUrl.toLowerCase();
  return COMPRESSION_EXTS.some((ext) => lower.endsWith(ext));
}

/** Parse an Armbian image filename into structured metadata across three conventions: Standard
 * `Armbian_{version}_{board}_...`, Labeled `Armbian_{label}_{version}_{board}_...` (label when parts[1] non-numeric), Prefixed `Armbian-unofficial_{version}_{board}_...`. */
export function parseArmbianFilename(filename: string): ArmbianFilenameInfo | null {
  const basename = filename.split('/').pop()?.split('\\').pop() ?? filename;

  // Strip compression extensions, then .img
  let name = basename;
  for (const ext of COMPRESSION_EXTS) {
    if (name.endsWith(ext)) {
      name = name.slice(0, -ext.length);
      break;
    }
  }
  if (name.endsWith('.img')) {
    name = name.slice(0, -4);
  }

  const parts = name.split('_');

  // Must start with "armbian" (possibly hyphenated, e.g. "Armbian-unofficial")
  if (parts.length < 4 || !parts[0].toLowerCase().startsWith('armbian')) {
    return null;
  }

  // If parts[1] doesn't start with a digit, it's a label (e.g. "community")
  const offset = parts[1] && !/^\d/.test(parts[1]) ? 1 : 0;

  // Need at least board index (2+offset) to exist
  if (parts.length < 3 + offset) {
    return null;
  }

  return {
    boardSlug: parts[2 + offset].toLowerCase(),
    version: parts[1 + offset] || null,
    distro: parts[3 + offset] || null,
    branch: parts[4 + offset] || null,
    kernel: parts[5 + offset] || null,
    desktop: parts.length > 6 + offset ? parts.slice(6 + offset).join('_') : null,
  };
}

/** Stable identity key for an Armbian image filename (board+version+distro+branch+kernel+desktop),
 * used to match a remote image against locally cached files regardless of compression extension.
 * Returns null when the name isn't a recognizable Armbian image. */
export function armbianIdentityKey(filename: string): string | null {
  const parsed = parseArmbianFilename(filename);
  if (!parsed) return null;
  return [parsed.boardSlug, parsed.version, parsed.distro, parsed.branch, parsed.kernel, parsed.desktop]
    .map((part) => (part ?? '').toLowerCase())
    .join('|');
}

/**
 * Split an Armbian version into its headline base and optional build/trunk suffix.
 * "26.2.0-trunk.904" -> { base: "26.2.0", build: "trunk.904" }; "26.5.1" -> { base: "26.5.1", build: "" }.
 */
export function splitArmbianVersion(version: string): { base: string; build: string } {
  const raw = version || '';
  const dash = raw.indexOf('-');
  return dash === -1 ? { base: raw, build: '' } : { base: raw.slice(0, dash), build: raw.slice(dash + 1) };
}

/** Format an ISO 8601 date as a short, locale-aware date (e.g. "29 May 2026"); undefined when unparseable. */
export function formatDate(iso: string, locale?: string): string | undefined {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return undefined;
  return parsed.toLocaleDateString(locale, { year: 'numeric', month: 'short', day: 'numeric' });
}

/** Strip ANSI escape sequences (terminal colour codes) from `text` so it copies/exports as plain text. */
export function stripAnsiCodes(text: string): string {
  // eslint-disable-next-line no-control-regex -- matching control chars is intentional
  const ansiEscapePattern = /[\u001b\u009b][[()#;?]*(?:[0-9]{1,4}(?:;[0-9]{0,4})*)?[0-9A-ORZcf-nqry=><]/g;
  return text.replace(ansiEscapePattern, '');
}

/** True when the string is a well-formed http(s) URL. */
export function isHttpUrl(value: string): boolean {
  try {
    const url = new URL(value.trim());
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch {
    return false;
  }
}

/** Strip a leading vendor name from a board name so a vendor kicker and the name don't repeat it. */
export function stripVendorPrefix(name: string, vendorName: string): string {
  if (!vendorName || !name.toLowerCase().startsWith(vendorName.toLowerCase())) return name;
  return name.slice(vendorName.length).trim() || name;
}

/** Branded OS identity (title + meta) for home OS row and flash header. API images use structured fields;
 * custom/cached parse distro_release filename — Armbian builds (incl. trunk/unofficial) yield version+variant (GNOME/Minimal/…), else raw filename with no meta. */
export function formatImageIdentity(
  image: ImageInfo,
  t: (key: string) => string
): { title: string; meta: string | null } {
  if (image.is_custom) {
    const parsed = parseArmbianFilename(image.distro_release || '');
    if (parsed?.version) {
      const version = splitArmbianVersion(parsed.version).base;
      const desktopEnv = parsed.desktop ? getDesktopEnv(parsed.desktop) : null;
      const variant =
        desktopEnv && DESKTOP_BADGES[desktopEnv] ? DESKTOP_BADGES[desktopEnv].label : t('modal.minimal');
      const os = parsed.distro ? getOsInfo(parsed.distro)?.name ?? null : null;
      return {
        title: `Armbian ${version} ${variant}`.replace(/\s+/g, ' ').trim(),
        meta: os && parsed.branch ? `${os} · ${parsed.branch}` : os || parsed.branch || null,
      };
    }
    return { title: image.distro_release || '', meta: null };
  }

  const version = splitArmbianVersion(image.release || '').base;
  const meta =
    image.distro_release && image.kernel_branch
      ? `${image.distro_release} · ${image.kernel_branch}`
      : image.distro_release || image.kernel_branch || null;
  return {
    title: `Armbian ${version} ${getImageVariantLabel(image, t)}`.replace(/\s+/g, ' ').trim(),
    meta: meta || null,
  };
}

/** Extract a message from an unknown error value, using `fallback` if none found */
export function getErrorMessage(error: unknown, fallback: string = 'An error occurred'): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return fallback;
}

/** Board sort comparator: by support tier, then alphabetically. */
export function compareBoardsBySupport<T extends {
  support_tier: string;
  name: string;
}>(a: T, b: T): number {
  const aIdx = SUPPORT_TIER_ORDER.indexOf(a.support_tier);
  const bIdx = SUPPORT_TIER_ORDER.indexOf(b.support_tier);
  const aPriority = aIdx === -1 ? SUPPORT_TIER_ORDER.length : aIdx;
  const bPriority = bIdx === -1 ? SUPPORT_TIER_ORDER.length : bIdx;
  if (aPriority !== bPriority) return aPriority - bPriority;
  return a.name.localeCompare(b.name);
}
