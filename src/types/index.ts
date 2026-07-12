export interface BoardInfo {
  slug: string;
  name: string;
  /** Vendor slug identifier (e.g., "radxa") */
  vendor: string;
  /** Vendor display name (e.g., "Radxa") */
  vendor_name: string;
  /** Support tier: "platinum", "standard", "community", "eos", "tvb", "wip" */
  support_tier: string;
  image_count: number;
  /** Whether desktop environment images are available */
  has_desktop: boolean;
  /** Whether this board is featured/promoted */
  promoted: boolean;
  /** System-on-Chip model (e.g., "RK3588") */
  soc?: string;
  /** CPU architecture (e.g., "arm64") */
  architecture?: string;
  /** Short board description */
  summary?: string;
  /** QDL/EDL flashing metadata, present only for Qualcomm EDL boards */
  qdl?: BoardQdl | null;
}

/** Slim QDL support info served with a board; `supported` drives the UI gate. */
export interface BoardQdl {
  /** Whether this build has a write path for the board's QDL storage. */
  supported: boolean;
  /** EDL-entry hint ("button"/"jumper") for the on-screen QDL instructions. */
  edl_entry: string;
}

export interface ImageInfo {
  /** Armbian release version (e.g., "24.02.0") */
  release: string;
  distro_release: string;
  kernel_branch: string;
  kernel_version: string;
  image_variant: string;
  preinstalled_application: string;
  promoted: boolean;
  file_url: string;
  /** Direct CDN download URL */
  direct_url: string;
  /** SHA256 checksum file URL */
  sha_url: string | null;
  file_size: number;
  /** Build date (ISO 8601), when available */
  build_date?: string | null;
  /** Stability level: "stable", "edge", "nightly" */
  stability: string;
  /** Image format: "sd" (block), "qdl" (Qualcomm EDL), "rootfs", "qemu", "hyperv" */
  format: string;
  /** Storage target ("ufs" = raw Firehose write to internal UFS via QDL), else null. */
  storage?: string | null;
  /** Companion files (bootloaders, firmware, etc.) */
  companions: CompanionInfo[];
  /** Display variant files for multi-panel devices */
  display_variants: DisplayVariantInfo[];
  // Custom image fields
  is_custom?: boolean;
  custom_path?: string;
}

/** How an image is written: raw block (dd), QDL TAR (Firehose rawprogram), or QDL UFS (raw Firehose write). */
export const FLASH_METHOD = {
  BLOCK: 'block',
  QDL: 'qdl',
  QDL_UFS: 'qdl-ufs',
} as const;

export type FlashMethod = (typeof FLASH_METHOD)[keyof typeof FLASH_METHOD];

/** Single source of truth for the write path of an image. UFS is detected by the
 *  storage field (the API ships it as format "sd"), QDL TAR by the "qdl" format. */
export function deriveFlashMethod(image: Pick<ImageInfo, 'format' | 'storage'>): FlashMethod {
  if (image.storage?.toLowerCase() === 'ufs') return FLASH_METHOD.QDL_UFS;
  if (image.format === 'qdl') return FLASH_METHOD.QDL;
  return FLASH_METHOD.BLOCK;
}

/** A flash method targets a Qualcomm EDL device (QDL TAR or raw UFS) rather than a block device. */
export function isEdlMethod(method: FlashMethod | null | undefined): boolean {
  return !!method && method !== FLASH_METHOD.BLOCK;
}

/** Whether an image flashes over EDL; the EDL-aware counterpart of a plain block write. */
export function isEdlImage(image: Pick<ImageInfo, 'format' | 'storage'>): boolean {
  return isEdlMethod(deriveFlashMethod(image));
}

/** Companion file info (bootloader, fip, recovery, etc.) */
export interface CompanionInfo {
  type_name: string;
  label: string;
  url: string;
  size_bytes: number;
}

/** Display variant for multi-panel devices */
export interface DisplayVariantInfo {
  label: string;
  url: string;
  size_bytes: number;
}

/** Vendor/manufacturer information from the API */
export interface VendorInfo {
  slug: string;
  name: string;
  logo_url?: string;
  website?: string;
  description?: string;
  board_count: number;
  partner_tier?: string;
}

export interface BlockDevice {
  path: string;
  name: string;
  size: number;
  size_formatted: string;
  model: string;
  is_removable: boolean;
  is_system: boolean;
  bus_type?: string;
  /** Whether the device is read-only (e.g., SD card with write-protect lock) */
  is_read_only?: boolean;
}

export interface DownloadProgress {
  total_bytes: number;
  downloaded_bytes: number;
  is_verifying_sha: boolean;
  is_decompressing: boolean;
  progress_percent: number;
  error: string | null;
}

export interface FlashProgress {
  total_bytes: number;
  written_bytes: number;
  verified_bytes: number;
  is_verifying: boolean;
  progress_percent: number;
  error: string | null;
  /** Whether the current operation is a QDL (Qualcomm EDL) flash */
  is_qdl_mode: boolean;
  /** Current QDL stage (e.g., "sahara", "firehose", "partition:boot.img") */
  qdl_stage: string | null;
  /** Total number of partitions to program in QDL mode */
  partitions_total: number;
  /** Number of partitions programmed so far in QDL mode */
  partitions_written: number;
}

/** Represents a Qualcomm device in EDL mode detected via USB */
export interface QdlDevice {
  serial: string;
  bus_id: string;
  device_address: number;
  description: string;
}

/** Manufacturer information for board categorization */
export interface Manufacturer {
  id: string;
  name: string;
  color: string;
  boardCount: number;
}

/** Filter type for the image list */
export type ImageFilterType = 'all' | 'recommended' | 'stable' | 'rolling' | 'apps' | 'barebone';

/** Selection step in the wizard flow */
export type SelectionStep = 'manufacturer' | 'board' | 'image' | 'device';

/** Custom image info from the file picker */
export interface CustomImageInfo {
  path: string;
  name: string;
  size: number;
}

/** One-shot classification of a picked custom image (board + QDL TAR + UFS build slug) */
export interface CustomImageClassification {
  board: BoardInfo | null;
  is_qdl: boolean;
  /** Board slug when the file is a UFS build of a UFS-capable QDL board, else null */
  ufs_board_slug: string | null;
}

/** Cached image metadata from the backend cache directory */
export interface CachedImageInfo {
  filename: string;
  path: string;
  size: number;
  /** Unix timestamp (seconds) of last use */
  last_used: number;
  /** Board slug extracted from filename */
  board_slug: string | null;
  /** Human-readable board name derived from slug */
  board_name: string | null;
}

/** Cache size split into flashable images and assets (board/vendor photos + API JSON) */
export interface CacheBreakdown {
  /** Bytes used by flashable .img files */
  images: number;
  /** Bytes used by cached assets (photos, API JSON) */
  assets: number;
  /** Sum of images + assets */
  total: number;
}

/** Board identification read from /etc/armbian-release */
export interface ArmbianReleaseInfo {
  board: string; // e.g., "orangepi-5" - Board identifier for matching
  board_name: string; // e.g., "Orange Pi 5" - Human-readable board name for display
}

/** Login shell for the first user provisioned via autoconfig */
export type UserShell = 'bash' | 'zsh';

/** Armbian first-boot autoconfig settings; all fields optional, only set/non-empty values
 * are written into the image's /root/.not_logged_in_yet file. */
export interface AutoconfigConfig {
  applyNetwork?: boolean;
  ethernetEnabled?: boolean;
  wifiEnabled?: boolean;
  wifiSsid?: string;
  wifiKey?: string;
  wifiCountryCode?: string;
  useStaticIp?: boolean;
  staticIp?: string;
  staticMask?: string;
  staticGateway?: string;
  staticDns?: string;
  locale?: string;
  timezone?: string;
  langBasedOnLocation?: boolean;
  rootPassword?: string;
  rootKeyUrl?: string;
  userName?: string;
  userPassword?: string;
  userKeyUrl?: string;
  userShell?: UserShell;
  userRealName?: string;
  remoteConfigUrl?: string;
}

/** A named, client-side autoconfig profile the user can select before flashing */
export interface AutoconfigProfile {
  id: string;
  name: string;
  /** Unix timestamp (ms) of last edit, used for sorting */
  updatedAt: number;
  config: AutoconfigConfig;
}

export type AutoconfigProfileChangeAction = 'created' | 'updated' | 'deleted';

/** Detail carried by the EVENTS.PROFILES_CHANGED CustomEvent */
export interface AutoconfigProfilesChangedDetail {
  id: string;
  action: AutoconfigProfileChangeAction;
}
