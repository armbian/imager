// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

import { COLORS, UI, SLUGS, SUPPORT_TIER, SUPPORT_TIER_ORDER, IMAGE_VARIANT, LOCAL_SOURCE_LABEL, PLATFORM, PLATFORM_UA, BYTES_PER_KB } from '../config';
import { getImageVariantLabel, getOsInfo } from '../config/os-info';
import { getVariantBadge, getKernelType, KERNEL_BADGES } from '../config/badges';
import { IMAGE_STABILITY, type ImageInfo, type BoardInfo, type Manufacturer } from '../types';

// Re-export color helpers from the dedicated color module
export { hexToRgb, hexToRgba, adjustBrightness, solidBadgeVars } from './color';

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
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(BYTES_PER_KB));
  return parseFloat((bytes / Math.pow(BYTES_PER_KB, i)).toFixed(1)) + ' ' + sizes[i];
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

/** Kernel-field suffix marking a UFS build in a parsed Armbian filename */
export const UFS_KERNEL_SUFFIX = '-ufs';

/** Split a parsed kernel version into its version and whether it carries the UFS suffix. */
export function splitUfsKernel(kernel: string | null): { kernel: string | null; isUfs: boolean } {
  if (!kernel || !kernel.toLowerCase().endsWith(UFS_KERNEL_SUFFIX)) return { kernel, isUfs: false };
  return { kernel: kernel.slice(0, -UFS_KERNEL_SUFFIX.length), isUfs: true };
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
export function formatDate(
  iso: string,
  locale?: string,
  month: 'short' | 'long' = 'short'
): string | undefined {
  const parsed = new Date(iso);
  if (Number.isNaN(parsed.getTime())) return undefined;
  return parsed.toLocaleDateString(locale, { year: 'numeric', month, day: 'numeric' });
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

export function trimmedOrUndefined(value?: string): string | undefined {
  return value?.trim() || undefined;
}

/** Parse a dotted-quad IPv4 address to its 32-bit value, or null when malformed. */
function parseIpv4(value: string): number | null {
  const parts = value.split('.');
  if (parts.length !== 4) return null;
  let n = 0;
  for (const part of parts) {
    if (!/^\d{1,3}$/.test(part) || Number(part) > 255) return null;
    n = n * 256 + Number(part);
  }
  return n;
}

/** Prefix length of a contiguous dotted-quad netmask (255.255.255.0 -> 24), or null when not a netmask. */
function netmaskPrefix(value: string): number | null {
  const n = parseIpv4(value);
  if (n === null) return null;
  const bits = n.toString(2).padStart(32, '0');
  if (!/^1*0*$/.test(bits)) return null;
  const firstZero = bits.indexOf('0');
  return firstZero === -1 ? 32 : firstZero;
}

/** Per-field i18n key (under settings.autoconfig) for an invalid static IP setting; absent fields are fine. */
export interface StaticIpErrors {
  ip?: string;
  mask?: string;
  gateway?: string;
  dns?: string;
}

/** Validate the static IP fields as a whole: well-formed values, a usable host address (not the subnet's network or
 * broadcast address) and a gateway inside that subnet. Empty fields are not flagged. */
export function staticIpErrors(ip?: string, mask?: string, gateway?: string, dns?: string): StaticIpErrors {
  const errors: StaticIpErrors = {};
  const addr = ip ? parseIpv4(ip) : null;
  const prefix = mask ? netmaskPrefix(mask) : null;
  if (ip && addr === null) errors.ip = 'ipInvalid';
  if (mask && prefix === null) errors.mask = 'maskInvalid';

  // Arithmetic, not bitwise: JS bitwise ops are signed 32-bit and break above 127.x.x.x.
  const size = prefix === null ? 0 : 2 ** (32 - prefix);
  if (addr !== null && prefix !== null && prefix <= 30) {
    const host = addr % size;
    if (host === 0 || host === size - 1) errors.ip = 'ipNotHost';
  }

  if (gateway) {
    const gw = parseIpv4(gateway);
    if (gw === null) errors.gateway = 'gatewayInvalid';
    else if (addr !== null && prefix !== null) {
      if (Math.floor(gw / size) !== Math.floor(addr / size)) errors.gateway = 'gatewayOutsideSubnet';
      else if (gw === addr) errors.gateway = 'gatewaySameAsIp';
    }
  }

  if (dns && dns.split(/[\s,]+/).filter(Boolean).some((d) => parseIpv4(d) === null)) errors.dns = 'dnsInvalid';
  return errors;
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
      const variant = getVariantBadge(parsed.desktop)?.label ?? t('modal.minimal');
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

/** Kernel branch label plus version, e.g. "Current 6.12.35"; null when the image carries neither. */
export function formatKernelLabel(image: ImageInfo): string | null {
  const type = getKernelType(image.kernel_branch);
  const label = type ? KERNEL_BADGES[type].label : image.kernel_branch;
  return `${label ?? ''} ${image.kernel_version ?? ''}`.trim() || null;
}

/** Extract a message from an unknown error value, using `fallback` if none found */
export function getErrorMessage(error: unknown, fallback: string = 'An error occurred'): string {
  if (error instanceof Error) return error.message;
  if (typeof error === 'string') return error;
  return fallback;
}

type LocalImageVariant = (typeof IMAGE_VARIANT)[keyof typeof IMAGE_VARIANT];

/** Synthetic ImageInfo for a local (cached or custom) image file; `storage` is set only when given. */
export function buildLocalImage({ variant, name, size, path, format, storage, supportsAutoconfig }: {
  variant: LocalImageVariant;
  name: string;
  size: number;
  path: string;
  format: string;
  storage?: string | null;
  supportsAutoconfig?: boolean;
}): ImageInfo {
  return {
    release: LOCAL_SOURCE_LABEL[variant],
    distro_release: name,
    kernel_branch: '',
    kernel_version: '',
    image_variant: variant,
    preinstalled_application: '',
    promoted: false,
    file_url: '',
    direct_url: '',
    sha_url: null,
    file_size: size,
    stability: IMAGE_STABILITY.STABLE,
    format,
    ...(storage !== undefined && { storage }),
    companions: [],
    display_variants: [],
    is_custom: true,
    custom_path: path,
    ...(supportsAutoconfig !== undefined && { supports_autoconfig: supportsAutoconfig }),
  };
}

/** Synthetic board for a local image that matched no API board. */
export function buildLocalBoard({ slug, name, vendor, vendorName }: {
  slug: string;
  name: string;
  vendor: string;
  vendorName: string;
}): BoardInfo {
  return {
    slug,
    name,
    vendor,
    vendor_name: vendorName,
    support_tier: SUPPORT_TIER.COMMUNITY,
    image_count: 1,
    has_desktop: false,
    promoted: false,
  };
}

/** Single-board manufacturer entry for a local image selection. */
export function localManufacturer(board: Pick<BoardInfo, 'vendor' | 'vendor_name'>): Manufacturer {
  return { id: board.vendor, name: board.vendor_name, color: DEFAULT_COLOR, boardCount: 1 };
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

/** Platform from the webview user agent, available before get_system_info answers. */
export function uiPlatform(): (typeof PLATFORM)[keyof typeof PLATFORM] {
  const ua = navigator.userAgent;
  if (ua.includes(PLATFORM_UA.MACOS)) return PLATFORM.MACOS;
  if (ua.includes(PLATFORM_UA.WINDOWS)) return PLATFORM.WINDOWS;
  return PLATFORM.LINUX;
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function tailPath(path: string, segments = 1): string {
  return path.split(/[\\/]/).filter(Boolean).slice(-segments).join('/') || path;
}
