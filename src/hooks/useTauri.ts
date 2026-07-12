import { invoke } from '@tauri-apps/api/core';
import type { BoardInfo, ImageInfo, BlockDevice, DownloadProgress, FlashProgress, CustomImageInfo, CustomImageClassification, ArmbianReleaseInfo, CachedImageInfo, CacheBreakdown, QdlDevice, VendorInfo, AutoconfigConfig } from '../types';

export async function getBoards(): Promise<BoardInfo[]> {
  return invoke('get_boards');
}

export async function getImagesForBoard(
  boardSlug: string,
  preappFilter?: string,
  kernelFilter?: string,
  variantFilter?: string,
  stability?: string
): Promise<ImageInfo[]> {
  return invoke('get_images_for_board', {
    boardSlug,
    preappFilter,
    kernelFilter,
    variantFilter,
    stability,
  });
}

export async function getVendors(): Promise<VendorInfo[]> {
  return invoke('get_vendors');
}

export async function getBlockDevices(): Promise<BlockDevice[]> {
  return invoke('get_block_devices');
}

export async function requestWriteAuthorization(devicePath: string): Promise<boolean> {
  return invoke('request_write_authorization', { devicePath });
}

export async function downloadImage(fileUrl: string, shaUrl?: string | null): Promise<string> {
  return invoke('download_image', { fileUrl, shaUrl });
}

export async function getDownloadProgress(): Promise<DownloadProgress> {
  return invoke('get_download_progress');
}

/** Flash an image to a device. With `autoconfig`, the Armbian first-boot file is written
 * into the image; omitting it keeps default behaviour. */
export async function flashImage(
  imagePath: string,
  devicePath: string,
  verify: boolean = true,
  autoconfig?: AutoconfigConfig | null
): Promise<void> {
  return invoke('flash_image', { imagePath, devicePath, verify, autoconfig });
}

export async function getFlashProgress(): Promise<FlashProgress> {
  return invoke('get_flash_progress');
}

export async function cancelOperation(): Promise<void> {
  return invoke('cancel_operation');
}

export async function deleteDownloadedImage(imagePath: string): Promise<void> {
  return invoke('delete_downloaded_image', { imagePath });
}

/** Force-delete a cached image (bypasses cache_enabled), for when a file looks corrupted */
export async function forceDeleteCachedImage(imagePath: string): Promise<void> {
  return invoke('force_delete_cached_image', { imagePath });
}

/** Continue a download without SHA verification, returning the decompressed image path */
export async function continueDownloadWithoutSha(): Promise<string> {
  return invoke('continue_download_without_sha');
}

/** Clean up the temp file left by a failed/cancelled download */
export async function cleanupFailedDownload(): Promise<void> {
  return invoke('cleanup_failed_download');
}

export async function deleteDecompressedCustomImage(imagePath: string): Promise<void> {
  return invoke('delete_decompressed_custom_image', { imagePath });
}

/** Detect board info from an Armbian image filename, or null if no match */
export async function detectBoardFromFilename(filename: string): Promise<BoardInfo | null> {
  return invoke('detect_board_from_filename', { filename });
}

/** Classify a picked custom image in one call: matched board, QDL TAR flag, and UFS build slug */
export async function classifyCustomImage(path: string): Promise<CustomImageClassification> {
  return invoke('classify_custom_image', { path });
}

// Re-export CustomImageInfo for backward compatibility
export type { CustomImageInfo } from '../types';

export async function selectCustomImage(): Promise<CustomImageInfo | null> {
  return invoke('select_custom_image');
}

export async function checkNeedsDecompression(imagePath: string): Promise<boolean> {
  return invoke('check_needs_decompression', { imagePath });
}

export async function decompressCustomImage(imagePath: string): Promise<string> {
  return invoke('decompress_custom_image', { imagePath });
}

export interface UploadResult {
  url: string;
  key: string;
}

export async function uploadLogs(): Promise<UploadResult> {
  return invoke('upload_logs');
}

export async function openUrl(url: string): Promise<void> {
  return invoke('open_url', { url });
}

export async function logInfo(module: string, message: string): Promise<void> {
  return invoke('log_from_frontend', { module, message });
}

export async function logWarn(module: string, message: string): Promise<void> {
  return invoke('log_warn_from_frontend', { module, message });
}

export interface GitHubRelease {
  tag_name: string;
  name: string;
  body: string | null;
  html_url: string;
  published_at: string;
}

/** Fetch GitHub release info for a version tag (e.g. "1.0.0" or "v1.0.0") */
export async function getGithubRelease(version: string): Promise<GitHubRelease> {
  return invoke('get_github_release', { version });
}

/** Whether the app runs from /Applications (always true off macOS); gates an update warning */
export async function isAppInApplications(): Promise<boolean> {
  return invoke('is_app_in_applications');
}

/** Get the real system platform and architecture */
export async function getSystemInfo(): Promise<{ platform: string; arch: string }> {
  return invoke('get_system_info');
}

/** Get the Tauri framework version */
export async function getTauriVersion(): Promise<string> {
  return invoke('get_tauri_version');
}

/** Get the log file contents (last 10k lines if over 5MB); ANSI colors preserved */
export async function getLogs(): Promise<string> {
  return invoke('get_logs');
}

// ============================================================================
// Cache Management
// ============================================================================

/** Get the cache size split into flashable images and assets (photos + API JSON) */
export async function getCacheBreakdown(): Promise<CacheBreakdown> {
  return invoke('get_cache_breakdown');
}

/** Clear all cached images (irreversible) */
export async function clearCache(): Promise<void> {
  return invoke('clear_cache');
}

/** List cached images with metadata (filename, size, last used, parsed board) */
export async function listCachedImages(): Promise<CachedImageInfo[]> {
  return invoke('list_cached_images');
}

/** Delete one cached image by filename, returning the new total cache size in bytes */
export async function deleteCachedImage(filename: string): Promise<number> {
  return invoke('delete_cached_image', { filename });
}

// ============================================================================
// Connectivity
// ============================================================================

/** Check reachability of the Armbian API (HEAD request, 5s timeout) */
export async function checkConnectivity(): Promise<boolean> {
  return invoke('check_connectivity');
}

// ============================================================================
// Picture Cache
// ============================================================================

/** Get a board image as a data URI from cache (downloads if needed), or null if unavailable */
export async function getCachedBoardImage(boardSlug: string): Promise<string | null> {
  return invoke('get_cached_board_image', { boardSlug });
}

/** Get a vendor logo as a data URI from cache (downloads if needed), or null if unavailable */
export async function getCachedVendorLogo(vendorSlug: string): Promise<string | null> {
  return invoke('get_cached_vendor_logo', { vendorSlug });
}

// ============================================================================
// Armbian System Detection
// ============================================================================

/** Detect an Armbian host via /etc/armbian-release (Linux only); null otherwise */
export async function getArmbianRelease(): Promise<ArmbianReleaseInfo | null> {
  return invoke('get_armbian_release');
}

// === QDL (Qualcomm EDL) operations ===

/** Detect Qualcomm devices connected via USB in EDL (Emergency Download) mode */
export async function getQdlDevices(): Promise<QdlDevice[]> {
  return invoke('get_qdl_devices');
}

/** Flash a QDL image (TAR archive) to a device in EDL mode; `serial` targets a specific device, `autoconfig` injects a profile */
export async function flashQdlImage(
  tarPath: string,
  serial?: string,
  autoconfig?: AutoconfigConfig | null
): Promise<void> {
  return invoke('flash_qdl_image', { tarPath, serial, autoconfig });
}

/** Flash a decompressed UFS .img to an EDL device via a raw Firehose write; the loader is
 * resolved from `soc`, then `boardSlug` (the API leaves `soc` null for these boards). */
export async function flashQdlUfsImage(
  imagePath: string,
  soc: string,
  boardSlug: string,
  serial?: string,
  autoconfig?: AutoconfigConfig | null
): Promise<void> {
  return invoke('flash_qdl_ufs_image', { imagePath, soc, boardSlug, serial, autoconfig });
}
