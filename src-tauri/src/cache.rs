//! Image cache management module
//!
//! Handles persistent caching of downloaded Armbian images with
//! configurable size limits and LRU (Least Recently Used) eviction.
//!
//! Thread Safety:
//! All cache operations are protected by a global Mutex to prevent
//! race conditions when multiple threads access the cache simultaneously.

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::SystemTime;

use filetime::FileTime;
use once_cell::sync::Lazy;

use crate::config;
use crate::utils::get_cache_dir;
use crate::{log_debug, log_error, log_info, log_warn};

const MODULE: &str = "cache";

/// Re-export default max cache size from config
pub use crate::config::cache::DEFAULT_MAX_SIZE;

/// Global mutex to ensure thread-safe cache operations
///
/// This prevents race conditions when multiple operations try to
/// read/write cache files simultaneously (e.g., eviction during download).
static CACHE_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

/// Cache entry with metadata for LRU eviction
#[derive(Debug)]
struct CacheEntry {
    path: PathBuf,
    size: u64,
    modified: SystemTime,
}

/// Get the image cache directory path
pub fn get_images_cache_dir() -> PathBuf {
    get_cache_dir(config::app::NAME).join("images")
}

/// Calculate total size of all cached images in bytes
///
/// Scans the images cache directory and sums up file sizes.
/// Returns 0 if directory doesn't exist or on error.
/// Thread-safe: acquires cache lock during operation.
pub fn calculate_cache_size() -> Result<u64, String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    calculate_cache_size_internal()
}

/// Internal implementation of calculate_cache_size without locking
///
/// Used by functions that already hold the cache lock.
fn calculate_cache_size_internal() -> Result<u64, String> {
    let cache_dir = get_images_cache_dir();

    if !cache_dir.exists() {
        log_debug!(MODULE, "Cache directory doesn't exist, size is 0");
        return Ok(0);
    }

    let entries = fs::read_dir(&cache_dir).map_err(|e| {
        log_error!(MODULE, "Failed to read cache directory: {}", e);
        format!("Failed to read cache directory: {}", e)
    })?;

    let mut total_size: u64 = 0;
    let mut file_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                total_size += metadata.len();
                file_count += 1;
            }
        }
    }

    log_debug!(
        MODULE,
        "Cache size: {} bytes ({} files)",
        total_size,
        file_count
    );

    Ok(total_size)
}

/// Get list of cached files sorted by modification time (oldest first)
///
/// Returns a vector of CacheEntry structs for LRU eviction.
/// Note: This function does not acquire the cache lock - caller must ensure thread safety.
fn get_cached_files_by_age_internal() -> Result<Vec<CacheEntry>, String> {
    let cache_dir = get_images_cache_dir();

    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&cache_dir).map_err(|e| {
        log_error!(MODULE, "Failed to read cache directory: {}", e);
        format!("Failed to read cache directory: {}", e)
    })?;

    let mut files: Vec<CacheEntry> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                let modified = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                files.push(CacheEntry {
                    path,
                    size: metadata.len(),
                    modified,
                });
            }
        }
    }

    // Sort by modification time (oldest first for LRU eviction)
    files.sort_by(|a, b| a.modified.cmp(&b.modified));

    Ok(files)
}

/// Evict oldest files until cache is under the specified limit
///
/// Uses LRU (Least Recently Used) strategy based on file modification time.
/// Thread-safe: acquires cache lock during operation.
pub fn evict_to_size(max_size: u64) -> Result<(), String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let current_size = calculate_cache_size_internal()?;

    if current_size <= max_size {
        log_debug!(
            MODULE,
            "Cache size {} is within limit {}, no eviction needed",
            current_size,
            max_size
        );
        return Ok(());
    }

    log_info!(
        MODULE,
        "Cache size {} exceeds limit {}, evicting oldest files",
        current_size,
        max_size
    );

    let files = get_cached_files_by_age_internal()?;
    let mut freed_space: u64 = 0;
    let target_free = current_size - max_size;

    for entry in files {
        if freed_space >= target_free {
            break;
        }

        log_info!(MODULE, "Evicting cached file: {}", entry.path.display());

        if let Err(e) = fs::remove_file(&entry.path) {
            log_warn!(MODULE, "Failed to remove cached file: {}", e);
            continue;
        }

        freed_space += entry.size;
    }

    log_info!(MODULE, "Evicted {} bytes from cache", freed_space);

    Ok(())
}

/// Clear all cached images
///
/// Removes all files from the images cache directory.
/// Thread-safe: acquires cache lock during operation.
pub fn clear_cache() -> Result<(), String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let cache_dir = get_images_cache_dir();

    if !cache_dir.exists() {
        log_info!(MODULE, "Cache directory doesn't exist, nothing to clear");
        return Ok(());
    }

    log_info!(MODULE, "Clearing cache directory: {}", cache_dir.display());

    let entries = fs::read_dir(&cache_dir).map_err(|e| {
        log_error!(MODULE, "Failed to read cache directory: {}", e);
        format!("Failed to read cache directory: {}", e)
    })?;

    let mut removed_count = 0;
    let mut failed_count = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            match fs::remove_file(&path) {
                Ok(()) => {
                    removed_count += 1;
                    log_debug!(MODULE, "Removed: {}", path.display());
                }
                Err(e) => {
                    failed_count += 1;
                    log_warn!(MODULE, "Failed to remove {}: {}", path.display(), e);
                }
            }
        }
    }

    log_info!(
        MODULE,
        "Cache cleared: {} files removed, {} failed",
        removed_count,
        failed_count
    );

    if failed_count > 0 {
        return Err(format!("Failed to remove {} cached files", failed_count));
    }

    Ok(())
}

/// Check if a cached image exists and return its path
///
/// Looks for a file with the given filename in the cache directory.
/// If found, updates the file's modification time to mark it as recently used.
/// Thread-safe: acquires cache lock during operation.
pub fn get_cached_image(filename: &str) -> Option<PathBuf> {
    let _lock = match CACHE_LOCK.lock() {
        Ok(guard) => guard,
        Err(e) => {
            log_error!(MODULE, "Failed to acquire cache lock: {}", e);
            return None;
        }
    };

    let cache_dir = get_images_cache_dir();
    let cached_path = cache_dir.join(filename);

    if cached_path.exists() && cached_path.is_file() {
        log_info!(MODULE, "Found cached image: {}", cached_path.display());

        // Touch the file to update modification time (for LRU)
        if let Err(e) = update_file_mtime(&cached_path) {
            log_warn!(MODULE, "Failed to update mtime for cached file: {}", e);
        }

        Some(cached_path)
    } else {
        log_debug!(MODULE, "Image not in cache: {}", filename);
        None
    }
}

/// Update file modification time to current time
///
/// Used for LRU tracking - accessed files get their mtime updated.
/// Uses the filetime crate for reliable cross-platform mtime updates.
fn update_file_mtime(path: &PathBuf) -> Result<(), String> {
    let now = FileTime::now();
    filetime::set_file_mtime(path, now)
        .map_err(|e| format!("Failed to update file mtime: {}", e))?;

    log_debug!(MODULE, "Updated mtime for: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cache_size_empty() {
        // For non-existent directory, should return 0
        let result = calculate_cache_size();
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_cached_files_by_age() {
        // Acquire lock and test internal function
        let _lock = CACHE_LOCK.lock().unwrap();
        let result = get_cached_files_by_age_internal();
        assert!(result.is_ok());
    }

    #[test]
    fn test_clear_cache_nonexistent() {
        // Should succeed even if directory doesn't exist
        let result = clear_cache();
        assert!(result.is_ok());
    }
}
