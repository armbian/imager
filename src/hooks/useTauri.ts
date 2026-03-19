import { invoke } from '@tauri-apps/api/core';
import type { BoardInfo, ImageInfo, BlockDevice, DownloadProgress, FlashProgress, CustomImageInfo, ArmbianReleaseInfo, CachedImageInfo } from '../types';

export async function getBoards(): Promise<BoardInfo[]> {
  return invoke('get_boards');
}

export async function getImagesForBoard(
  boardSlug: string,
  preappFilter?: string,
  kernelFilter?: string,
  variantFilter?: string,
  stableOnly: boolean = false
): Promise<ImageInfo[]> {
  return invoke('get_images_for_board', {
    boardSlug,
    preappFilter,
    kernelFilter,
    variantFilter,
    stableOnly,
  });
}

export async function getBlockDevices(): Promise<BlockDevice[]> {
  return invoke('get_block_devices');
}

export async function requestWriteAuthorization(devicePath: string): Promise<boolean> {
  return invoke('request_write_authorization', { devicePath });
}

export async function downloadImage(fileUrl: string, fileUrlSha?: string | null): Promise<string> {
  return invoke('download_image', { fileUrl, fileUrlSha });
}

export async function getDownloadProgress(): Promise<DownloadProgress> {
  return invoke('get_download_progress');
}

export async function flashImage(
  imagePath: string,
  devicePath: string,
  verify: boolean = true
): Promise<void> {
  return invoke('flash_image', { imagePath, devicePath, verify });
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

/**
 * Force delete a cached image regardless of cache settings
 *
 * Used when an image repeatedly fails to flash, suggesting the cached
 * file may be corrupted. Bypasses the cache_enabled check.
 *
 * @param imagePath - Path to the cached image file
 * @throws Error if deletion fails or path is outside cache directory
 */
export async function forceDeleteCachedImage(imagePath: string): Promise<void> {
  return invoke('force_delete_cached_image', { imagePath });
}

/**
 * Continue download without SHA verification (uses already downloaded file)
 * Called when user confirms to proceed after SHA unavailable error
 *
 * @returns Promise resolving to the path of the decompressed image
 * @throws Error if no pending download or decompression fails
 */
export async function continueDownloadWithoutSha(): Promise<string> {
  return invoke('continue_download_without_sha');
}

/**
 * Clean up temp file from a failed download
 * Called when user cancels after SHA unavailable error
 */
export async function cleanupFailedDownload(): Promise<void> {
  return invoke('cleanup_failed_download');
}

export async function deleteDecompressedCustomImage(imagePath: string): Promise<void> {
  return invoke('delete_decompressed_custom_image', { imagePath });
}

/**
 * Detects board information from custom image filename
 * Parses Armbian naming convention to extract board slug and match against database
 *
 * @param filename - Filename (can include path)
 * @returns Promise resolving to BoardInfo if detected, null otherwise
 */
export async function detectBoardFromFilename(filename: string): Promise<BoardInfo | null> {
  return invoke('detect_board_from_filename', { filename });
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

export async function logDebug(module: string, message: string): Promise<void> {
  return invoke('log_debug_from_frontend', { module, message });
}

export interface GitHubRelease {
  tag_name: string;
  name: string;
  body: string | null;
  html_url: string;
  published_at: string;
}

/**
 * Fetches GitHub release information for a specific version
 *
 * @param version - Version tag (e.g., "1.0.0" or "v1.0.0")
 * @returns Promise resolving to GitHub release data with notes, date, and URL
 */
export async function getGithubRelease(version: string): Promise<GitHubRelease> {
  return invoke('get_github_release', { version });
}

/**
 * Checks if the app is running from /Applications directory (macOS only)
 * Used to show a specific error message when updates fail outside /Applications.
 * @returns true if in /Applications or on non-macOS, false otherwise
 */
export async function isAppInApplications(): Promise<boolean> {
  return invoke('is_app_in_applications');
}

/**
 * Get the real system platform and architecture
 */
export async function getSystemInfo(): Promise<{ platform: string; arch: string }> {
  return invoke('get_system_info');
}

/**
 * Get the Tauri framework version
 */
export async function getTauriVersion(): Promise<string> {
  return invoke('get_tauri_version');
}

/**
 * Get the current log file contents
 *
 * Retrieves the contents of the current log file. For large log files (>5MB),
 * only the last 10,000 lines are returned to prevent memory issues.
 *
 * @returns Promise resolving to the log file contents with ANSI color codes preserved
 * @throws Error if log file cannot be read or does not exist
 *
 * @example
 * // Display full log contents
 * const logs = await getLogs();
 * console.log(logs); // Full log contents with colors
 *
 * @example
 * // Handle errors gracefully
 * try {
 *   const logs = await getLogs();
 *   // Process logs...
 * } catch (error) {
 *   console.error('Failed to retrieve logs:', error);
 * }
 */
export async function getLogs(): Promise<string> {
  return invoke('get_logs');
}

// ============================================================================
// Cache Management
// ============================================================================

/**
 * Get the current cache size in bytes
 *
 * Calculates the total size of all cached images in the cache directory.
 *
 * @returns Promise resolving to cache size in bytes
 * @throws Error if cache size cannot be calculated
 */
export async function getCacheSize(): Promise<number> {
  return invoke('get_cache_size');
}

/**
 * Clear all cached images
 *
 * Removes all files from the image cache directory.
 * This is an irreversible operation.
 *
 * @throws Error if cache cannot be cleared
 */
export async function clearCache(): Promise<void> {
  return invoke('clear_cache');
}

/**
 * List all cached images with metadata
 *
 * Returns information about each cached file including filename, size,
 * last used timestamp, and board association parsed from filename.
 *
 * @returns Promise resolving to array of cached image metadata
 * @throws Error if cache directory cannot be read
 */
export async function listCachedImages(): Promise<CachedImageInfo[]> {
  return invoke('list_cached_images');
}

/**
 * Delete a single cached image by filename
 *
 * Validates the filename is within the cache directory, deletes the file,
 * and returns the updated total cache size in bytes.
 *
 * @param filename - Name of the cached file to delete
 * @returns Promise resolving to the new total cache size in bytes
 * @throws Error if file not found or deletion fails
 */
export async function deleteCachedImage(filename: string): Promise<number> {
  return invoke('delete_cached_image', { filename });
}

// ============================================================================
// Connectivity
// ============================================================================

/**
 * Check if the app can reach the Armbian API
 *
 * Performs a HEAD request with a 5-second timeout.
 * Returns true if online, false if offline.
 */
export async function checkConnectivity(): Promise<boolean> {
  return invoke('check_connectivity');
}

// ============================================================================
// Picture Cache
// ============================================================================

/**
 * Get a board image from local cache, downloading if needed
 *
 * Returns a data URI (data:image/png;base64,...) for the cached image, or null
 * if the image is unavailable (offline + not cached).
 */
export async function getCachedBoardImage(boardSlug: string): Promise<string | null> {
  return invoke('get_cached_board_image', { boardSlug });
}

/**
 * Get a vendor logo from local cache, downloading if needed
 *
 * Returns a data URI (data:image/png;base64,...) for the cached logo, or null
 * if the logo is unavailable (offline + not cached).
 */
export async function getCachedVendorLogo(vendorId: string, logoUrl: string): Promise<string | null> {
  return invoke('get_cached_vendor_logo', { vendorId, logoUrl });
}

// ============================================================================
// Armbian System Detection
// ============================================================================

/**
 * Detect if the app is running on an Armbian system
 *
 * Reads /etc/armbian-release (Linux only) to identify the current board.
 * Returns null if not on Armbian or on non-Linux platforms.
 *
 * @returns Promise resolving to ArmbianReleaseInfo if on Armbian, null otherwise
 *
 * @example
 * // Detect board
 * const info = await getArmbianRelease();
 * if (info) {
 *   console.log(`Running on: ${info.board_name} (${info.board})`);
 * } else {
 *   console.log('Not running on Armbian');
 * }
 */
export async function getArmbianRelease(): Promise<ArmbianReleaseInfo | null> {
  return invoke('get_armbian_release');
}
