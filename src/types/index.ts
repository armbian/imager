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
  /** Stability level: "stable", "edge", "nightly" */
  stability: string;
  /** Image format: "sd" (block), "qdl" (Qualcomm EDL), "rootfs", "qemu", "hyperv" */
  format: string;
  /** Companion files (bootloaders, firmware, etc.) */
  companions: CompanionInfo[];
  /** Display variant files for multi-panel devices */
  display_variants: DisplayVariantInfo[];
  // Custom image fields
  is_custom?: boolean;
  custom_path?: string;
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

/**
 * Manufacturer information for board categorization
 */
export interface Manufacturer {
  id: string;
  name: string;
  color: string;
  boardCount: number;
}

/**
 * Filter type for image list
 */
export type ImageFilterType = 'all' | 'recommended' | 'stable' | 'nightly' | 'apps' | 'barebone';

/**
 * Selection step in the wizard flow
 */
export type SelectionStep = 'manufacturer' | 'board' | 'image' | 'device';

/**
 * Modal type for app navigation (includes 'none' for closed state)
 */
export type ModalType = 'none' | SelectionStep;

/**
 * Custom image info from file picker
 */
export interface CustomImageInfo {
  path: string;
  name: string;
  size: number;
}

/**
 * Cached image metadata from the backend cache directory
 */
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

/**
 * Armbian system information from /etc/armbian-release
 *
 * Contains essential information about the currently running Armbian system.
 * This is populated when the app is running on an Armbian installation.
 */
export interface ArmbianReleaseInfo {
  board: string; // e.g., "orangepi-5" - Board identifier for matching
  board_name: string; // e.g., "Orange Pi 5" - Human-readable board name for display
}
