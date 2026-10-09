// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2025-2026 Daniele Briguglio, superkali@armbian.com

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
  /** Custom files only: classify_custom_image found an Armbian first-boot setup a profile can use */
  supports_autoconfig?: boolean;
}

/** API image formats the app writes; mirrors the allowlist in src-tauri/src/images/filters.rs */
export const IMAGE_FORMAT = {
  SD: 'sd',
  BLOCK: 'block',
  QDL: 'qdl',
} as const;

/** API storage target of an image; UFS builds ship as format "sd" */
export const IMAGE_STORAGE = {
  UFS: 'ufs',
} as const;

export const IMAGE_STABILITY = {
  STABLE: 'stable',
} as const;

/** Whether an image targets UFS storage (written raw over QDL/Firehose). */
export function isUfsImage(image: Pick<ImageInfo, 'storage'>): boolean {
  return image.storage?.toLowerCase() === IMAGE_STORAGE.UFS;
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
  if (isUfsImage(image)) return FLASH_METHOD.QDL_UFS;
  if (image.format === IMAGE_FORMAT.QDL) return FLASH_METHOD.QDL;
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
  /** Device path (`qdl://<bus_id>/<address>`) passed back to target this device */
  path: string;
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

/** One-shot classification of a picked custom image (board + QDL TAR + UFS build slug + profiles) */
export interface CustomImageClassification {
  board: BoardInfo | null;
  is_qdl: boolean;
  /** Board slug when the file is a UFS build of a UFS-capable QDL board, else null */
  ufs_board_slug: string | null;
  /** Matched board, Armbian file name (.img/.img.xz), or a raw .img with the first-login script */
  supports_autoconfig: boolean;
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

/** Where a host detection run stopped: the detection mode it applied, or why it applied none */
export type ArmbianDetectionOutcome = 'modal' | 'auto' | 'disabled' | 'notArmbian' | 'unknownBoard' | 'failed';

/** Where a profile's public SSH keys come from (lookup_ssh_keys) */
export type SshKeySource = 'github' | 'gitlab' | 'url';

/** One public key served by a key source */
export interface SshKeyInfo {
  /** OpenSSH type string, e.g. "ssh-ed25519" */
  keyType: string;
  /** Label as `ssh-keygen -l` prints it: ED25519, RSA, ECDSA, ED25519-SK, ECDSA-SK */
  label: string;
  comment: string | null;
  /** "SHA256:<base64>", as `ssh-keygen -l` prints it */
  fingerprint: string;
}

/** Result of lookup_ssh_keys; `keys` lists at most 50, `total` counts every valid key */
export interface SshKeyLookup {
  total: number;
  keys: SshKeyInfo[];
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

/** How a settled flash screen leaves: Back on error, Flash another on done */
export type FlashExit = () => void | Promise<void>;

// === Dev scenarios emulator (debug builds only; mirrors src-tauri/src/dev_scenarios/model.rs) ===

export type DevFlashOutcome = 'success' | 'writeError' | 'verifyMismatch' | 'unplug' | 'authDenied';
export type DevApiFault = 'normal' | 'offline' | 'slow' | 'empty' | 'serverError';
export type DevDownloadFault = 'normal' | 'offline' | 'slow' | 'shaMismatch';

/** A `vdisk` device takes its size from the backing file (sizeBytes 0). */
export interface DevFakeDevice {
  id: string;
  model?: string;
  sizeBytes?: number;
  busType?: 'SD' | 'USB' | 'NVMe' | 'SATA' | 'SAS' | null;
  isRemovable?: boolean;
  isSystem?: boolean;
  isReadOnly?: boolean;
  appearAfterMs?: number | null;
  disappearAfterMs?: number | null;
  vdisk?: boolean;
}

export interface DevFakeEdlDevice {
  id: string;
  serial?: string;
  description?: string;
  appearAfterMs?: number | null;
  disappearAfterMs?: number | null;
}

export interface DevFlashSim {
  outcome: DevFlashOutcome;
  failAtPercent: number;
  writeMbPerSec: number;
  verifyMbPerSec: number;
  authDelayMs: number;
}

export interface DevNetworkSim {
  api: DevApiFault;
  apiDelayMs: number;
  download: DevDownloadFault;
  downloadKbPerSec: number;
}

export interface DevScenario {
  hideRealDevices: boolean;
  devices: DevFakeDevice[];
  edlDevices: DevFakeEdlDevice[];
  flash: DevFlashSim;
  network: DevNetworkSim;
  /** What getArmbianRelease reports instead of /etc/armbian-release, on every platform */
  armbianHost: ArmbianReleaseInfo | null;
}

export interface DevPreset {
  id: string;
  label: string;
  description: string;
  scenario: DevScenario;
}

export interface DevLimits {
  idMaxLen: number;
  maxFakeDevices: number;
  maxTestImageMb: number;
  maxVdiskMb: number;
  maxDelayMs: number;
}

export interface DevVdisk {
  id: string;
  sizeBytes: number;
  path: string;
}

export interface DevScenariosStatus {
  active: boolean;
  scenario: DevScenario;
  /** The hot-plug clock: milliseconds since the scenario was set */
  elapsedMs: number;
  /** Fake device ids a simulated mid-write unplug removed */
  unplugged: string[];
  presets: DevPreset[];
  limits: DevLimits;
  vdiskSupported: boolean;
  vdisks: DevVdisk[];
}

export interface DevTestImage {
  path: string;
  sizeBytes: number;
}
