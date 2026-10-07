// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Persistent cache for downloaded Armbian images with size limits and LRU
//! eviction. All operations are guarded by a global Mutex.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use filetime::FileTime;
use once_cell::sync::Lazy;

use crate::utils::{assets_dir, images_dir, parse_armbian_filename, validate_cache_path};
use crate::{log_debug, log_error, log_info, log_warn};

const MODULE: &str = "cache";

pub use crate::config::cache::DEFAULT_MAX_SIZE;

/// Serializes cache operations, e.g. eviction racing a download
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
    images_dir()
}

/// Total size of all cached images in bytes (0 if the directory is missing)
pub fn calculate_cache_size() -> Result<u64, String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    calculate_cache_size_internal(&images_dir(), &assets_dir())
}

/// Cache size split by category: flashable images vs assets (board/vendor
/// photos and API JSON). `total` is the sum of the two.
#[derive(serde::Serialize)]
pub struct CacheBreakdown {
    pub images: u64,
    pub assets: u64,
    pub total: u64,
}

/// Cache size broken down into the flashable-image cache and the assets cache.
pub fn calculate_cache_breakdown() -> Result<CacheBreakdown, String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let images = recursive_dir_size(&images_dir());
    let assets = recursive_dir_size(&assets_dir());

    Ok(CacheBreakdown {
        images,
        assets,
        total: images + assets,
    })
}

/// Recursively sum the byte size of every file under `dir` (0 if absent).
/// Unreadable entries are skipped rather than failing the whole walk.
fn recursive_dir_size(dir: &Path) -> u64 {
    if !dir.exists() {
        return 0;
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log_warn!(MODULE, "Failed to read directory {}: {}", dir.display(), e);
            return 0;
        }
    };

    let mut total_size: u64 = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total_size += recursive_dir_size(&path);
        } else if path.is_file() {
            if let Ok(metadata) = fs::metadata(&path) {
                total_size += metadata.len();
            }
        }
    }

    total_size
}

/// Sum of the OS image cache and the assets cache. Caller must hold the cache lock.
fn calculate_cache_size_internal(images: &Path, assets: &Path) -> Result<u64, String> {
    let images_size = recursive_dir_size(images);
    let assets_size = recursive_dir_size(assets);
    let total_size = images_size + assets_size;

    log_debug!(
        MODULE,
        "Cache size: {} bytes (images: {}, assets: {})",
        total_size,
        images_size,
        assets_size
    );

    Ok(total_size)
}

/// Cached files sorted oldest-first for LRU eviction.
/// Caller must hold the cache lock; this function does not.
fn get_cached_files_by_age_internal(cache_dir: &Path) -> Result<Vec<CacheEntry>, String> {
    if !cache_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(cache_dir).map_err(|e| {
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

    files.sort_by_key(|a| a.modified);

    Ok(files)
}

/// Evict oldest files (by mtime) until the cache is under the given limit
pub fn evict_to_size(max_size: u64) -> Result<(), String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    evict_to_size_internal(&images_dir(), &assets_dir(), max_size)
}

/// Counts both dirs toward the limit but deletes only from `images`; caller holds the cache lock.
fn evict_to_size_internal(images: &Path, assets: &Path, max_size: u64) -> Result<(), String> {
    let current_size = calculate_cache_size_internal(images, assets)?;

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

    let files = get_cached_files_by_age_internal(images)?;
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

/// Recursively delete the contents of `dir`, leaving `dir` itself in place.
/// Returns (removed_files, failed_files).
fn clear_dir_contents(dir: &Path) -> (u32, u32) {
    let mut removed = 0;
    let mut failed = 0;

    if !dir.exists() {
        return (removed, failed);
    }

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) => {
            log_warn!(MODULE, "Failed to read directory {}: {}", dir.display(), e);
            return (removed, failed);
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let (sub_removed, sub_failed) = clear_dir_contents(&path);
            removed += sub_removed;
            failed += sub_failed;
            if let Err(e) = fs::remove_dir(&path) {
                log_warn!(MODULE, "Failed to remove dir {}: {}", path.display(), e);
            }
        } else {
            match fs::remove_file(&path) {
                Ok(()) => {
                    removed += 1;
                    log_debug!(MODULE, "Removed: {}", path.display());
                }
                Err(e) => {
                    failed += 1;
                    log_warn!(MODULE, "Failed to remove {}: {}", path.display(), e);
                }
            }
        }
    }

    (removed, failed)
}

/// Remove all cached data (OS images and assets). Leaves logs, transient flash
/// dirs, and the settings store untouched.
pub fn clear_cache() -> Result<(), String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let result = clear_cache_internal(&images_dir(), &assets_dir());

    // Drop picture_cache metadata so the next request reloads from empty disk.
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async {
            crate::picture_cache::reset_meta().await;
        });
    }

    result
}

/// Caller must hold the cache lock.
fn clear_cache_internal(images: &Path, assets: &Path) -> Result<(), String> {
    log_info!(
        MODULE,
        "Clearing cache: images={}, assets={}",
        images.display(),
        assets.display()
    );

    let (img_removed, img_failed) = clear_dir_contents(images);
    let (asset_removed, asset_failed) = clear_dir_contents(assets);

    let removed_count = img_removed + asset_removed;
    let failed_count = img_failed + asset_failed;

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

/// Return a cached image's path if present, bumping its mtime to mark it recently used
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

        // Bump mtime so LRU treats this as recently used.
        if let Err(e) = update_file_mtime(&cached_path) {
            log_warn!(MODULE, "Failed to update mtime for cached file: {}", e);
        }

        Some(cached_path)
    } else {
        log_debug!(MODULE, "Image not in cache: {}", filename);
        None
    }
}

/// Bump a file's mtime to now for LRU tracking (via the filetime crate)
fn update_file_mtime(path: &PathBuf) -> Result<(), String> {
    let now = FileTime::now();
    filetime::set_file_mtime(path, now)
        .map_err(|e| format!("Failed to update file mtime: {}", e))?;

    log_debug!(MODULE, "Updated mtime for: {}", path.display());
    Ok(())
}

/// Cached image metadata for frontend display, with board parsed from the filename
#[derive(Debug, Clone, serde::Serialize)]
pub struct CachedImageInfo {
    pub filename: String,
    pub path: String,
    pub size: u64,
    /// Unix timestamp (seconds) of last use/modification
    pub last_used: u64,
    /// Board slug extracted from filename (e.g., "orangepi-5")
    pub board_slug: Option<String>,
    /// Human-readable board name derived from slug
    pub board_name: Option<String>,
}

/// Convert a board slug to a display name, e.g. "nanopi-m5" -> "Nanopi M5"
fn slug_to_display_name(slug: &str) -> String {
    slug.split('-')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// List cached images with metadata, including the board parsed from each filename
pub fn list_cached_images() -> Result<Vec<CachedImageInfo>, String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let cache_dir = get_images_cache_dir();

    if !cache_dir.exists() {
        log_debug!(
            MODULE,
            "Cache directory doesn't exist, returning empty list"
        );
        return Ok(Vec::new());
    }

    let entries = fs::read_dir(&cache_dir).map_err(|e| {
        log_error!(MODULE, "Failed to read cache directory: {}", e);
        format!("Failed to read cache directory: {}", e)
    })?;

    let mut images: Vec<CachedImageInfo> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let metadata = match fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let last_used = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let parsed = parse_armbian_filename(&filename);
        let board_slug = parsed.map(|info| info.board_slug);
        let board_name = board_slug.as_deref().map(slug_to_display_name);

        images.push(CachedImageInfo {
            filename,
            path: path.to_string_lossy().to_string(),
            size: metadata.len(),
            last_used,
            board_slug,
            board_name,
        });
    }

    // Sort by board_slug (None last), then by filename
    images.sort_by(|a, b| match (&a.board_slug, &b.board_slug) {
        (None, None) => a.filename.cmp(&b.filename),
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (Some(_), None) => std::cmp::Ordering::Less,
        (Some(a_slug), Some(b_slug)) => {
            let slug_cmp = a_slug.cmp(b_slug);
            if slug_cmp == std::cmp::Ordering::Equal {
                a.filename.cmp(&b.filename)
            } else {
                slug_cmp
            }
        }
    });

    log_info!(MODULE, "Listed {} cached images", images.len());
    Ok(images)
}

/// Delete one cached image by filename (rejects path traversal), returning the
/// new total cache size
pub fn delete_cached_image(filename: &str) -> Result<u64, String> {
    let _lock = CACHE_LOCK
        .lock()
        .map_err(|e| format!("Failed to acquire cache lock: {}", e))?;

    let cache_dir = get_images_cache_dir();
    let file_path = cache_dir.join(filename);

    if filename.contains("..") || filename.contains('/') || filename.contains('\\') {
        log_error!(
            MODULE,
            "Invalid filename (path traversal attempt): {}",
            filename
        );
        return Err("Invalid filename".to_string());
    }

    if !file_path.exists() {
        return Err(format!("File not found in cache: {}", filename));
    }

    if !file_path.is_file() {
        return Err(format!("Not a file: {}", filename));
    }

    // Confirm the canonical path is still inside the cache directory.
    if let Err(e) = validate_cache_path(&file_path) {
        log_error!(
            MODULE,
            "Attempted to delete file outside cache: {}: {}",
            filename,
            e
        );
        return Err(e);
    }

    fs::remove_file(&file_path).map_err(|e| {
        log_error!(MODULE, "Failed to delete cached image {}: {}", filename, e);
        format!("Failed to delete image: {}", e)
    })?;

    log_info!(MODULE, "Deleted cached image: {}", filename);

    calculate_cache_size_internal(&cache_dir, &assets_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    struct Fixture {
        _root: tempfile::TempDir,
        images: PathBuf,
        assets: PathBuf,
    }

    fn fixture() -> Fixture {
        let root = tempfile::tempdir().unwrap();
        let images = root.path().join("images");
        let assets = root.path().join("assets");
        fs::create_dir_all(&images).unwrap();
        fs::create_dir_all(assets.join("boards")).unwrap();
        Fixture {
            _root: root,
            images,
            assets,
        }
    }

    fn write_aged(path: &Path, len: usize, age_secs: u64) {
        fs::write(path, vec![0u8; len]).unwrap();
        let mtime = SystemTime::now() - Duration::from_secs(age_secs);
        filetime::set_file_mtime(path, FileTime::from_system_time(mtime)).unwrap();
    }

    #[test]
    fn size_sums_images_and_nested_assets() {
        let f = fixture();
        fs::write(f.images.join("a.img.xz"), vec![0u8; 100]).unwrap();
        fs::write(f.images.join("b.img.xz"), vec![0u8; 50]).unwrap();
        fs::write(f.assets.join("boards").join("rock-5b.png"), vec![0u8; 7]).unwrap();
        assert_eq!(calculate_cache_size_internal(&f.images, &f.assets), Ok(157));
    }

    #[test]
    fn size_of_missing_dirs_is_zero() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        assert_eq!(calculate_cache_size_internal(&missing, &missing), Ok(0));
    }

    #[test]
    fn files_by_age_are_oldest_first_and_skip_dirs() {
        let f = fixture();
        write_aged(&f.images.join("new.img.xz"), 1, 10);
        write_aged(&f.images.join("old.img.xz"), 2, 300);
        write_aged(&f.images.join("mid.img.xz"), 3, 100);
        fs::create_dir(f.images.join("subdir")).unwrap();

        let names: Vec<_> = get_cached_files_by_age_internal(&f.images)
            .unwrap()
            .into_iter()
            .map(|e| e.path.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["old.img.xz", "mid.img.xz", "new.img.xz"]);
    }

    #[test]
    fn files_by_age_of_missing_dir_is_empty() {
        let root = tempfile::tempdir().unwrap();
        let entries = get_cached_files_by_age_internal(&root.path().join("missing")).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn eviction_drops_oldest_images_until_under_the_limit() {
        let f = fixture();
        write_aged(&f.images.join("old.img.xz"), 100, 300);
        write_aged(&f.images.join("mid.img.xz"), 100, 200);
        write_aged(&f.images.join("new.img.xz"), 100, 10);
        fs::write(f.assets.join("boards").join("x.png"), vec![0u8; 50]).unwrap();

        evict_to_size_internal(&f.images, &f.assets, 200).unwrap();

        assert!(!f.images.join("old.img.xz").exists());
        assert!(!f.images.join("mid.img.xz").exists());
        assert!(f.images.join("new.img.xz").exists());
        assert!(f.assets.join("boards").join("x.png").exists());
    }

    #[test]
    fn clear_removes_files_but_keeps_the_dirs() {
        let f = fixture();
        fs::write(f.images.join("a.img.xz"), b"img").unwrap();
        fs::write(f.images.join("b.img.xz.downloading"), b"part").unwrap();
        fs::write(f.assets.join("boards").join("rock-5b.png"), b"png").unwrap();

        clear_cache_internal(&f.images, &f.assets).unwrap();

        assert!(f.images.is_dir());
        assert!(f.assets.is_dir());
        assert_eq!(fs::read_dir(&f.images).unwrap().count(), 0);
        assert_eq!(fs::read_dir(&f.assets).unwrap().count(), 0);
    }

    #[test]
    fn clear_of_missing_dirs_is_ok() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        assert!(clear_cache_internal(&missing, &missing).is_ok());
        assert!(!missing.exists());
    }
}
