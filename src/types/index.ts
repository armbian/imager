export interface BoardInfo {
  slug: string;
  name: string;
  vendor: string;
  vendor_name: string;
  vendor_logo: string | null;
  image_count: number;
  has_standard_support: boolean;
  has_community_support: boolean;
  has_platinum_support: boolean;
  has_eos_support: boolean;
  has_tvb_support: boolean;
  has_wip_support: boolean;
}

export interface ImageInfo {
  armbian_version: string;
  distro_release: string;
  kernel_branch: string;
  image_variant: string;
  preinstalled_application: string;
  promoted: boolean;
  file_url: string;
  file_url_sha: string | null;
  file_size: number;
  download_repository: string;
  // Custom image fields
  is_custom?: boolean;
  custom_path?: string;
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
