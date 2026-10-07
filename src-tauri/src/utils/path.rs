// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Path manipulation helpers used across the application.

use std::path::{Path, PathBuf};

use super::app_cache_dir;
use crate::{log_info, log_warn};

/// Validate that a path resolves to within the cache directory, returning its
/// canonical form. Canonicalizes both paths to defeat symlink/traversal tricks.
pub fn validate_cache_path(path: &Path) -> Result<PathBuf, String> {
    validate_path_in_cache(path, &app_cache_dir())
}

pub fn validate_path_in_cache(path: &Path, cache_dir: &Path) -> Result<PathBuf, String> {
    let cache_dir = cache_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve cache directory: {}", e))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|e| format!("Failed to resolve path: {}", e))?;
    if !canonical_path.starts_with(&cache_dir) {
        return Err("Cannot operate on files outside cache directory".to_string());
    }
    Ok(canonical_path)
}

pub fn image_size(path: &Path) -> Result<u64, String> {
    std::fs::metadata(path)
        .map(|m| m.len())
        .map_err(|e| format!("Failed to get image size: {e}"))
}

/// Strip a compression extension (.xz, .gz, .bz2, .zst), returning the original if none matches
pub fn strip_compression_ext(filename: &str) -> &str {
    for ext in &[".xz", ".gz", ".bz2", ".zst"] {
        if let Some(stripped) = filename.strip_suffix(ext) {
            return stripped;
        }
    }
    filename
}

/// `<pid>.<nanos>`, so concurrent and repeated runs never pick the same temp name.
pub fn unique_suffix() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{}.{}", std::process::id(), nanos)
}

// Temp names from unique_suffix start with `<pid>.`.
fn owner_pid(name: &str) -> Option<u32> {
    name.split_once('.')?.0.parse().ok()
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn process_alive(pid: u32) -> bool {
    let Ok(pid) = libc::pid_t::try_from(pid) else {
        return false;
    };
    // Signal 0 only checks that the process exists; EPERM means it exists under another user.
    let exists = unsafe { libc::kill(pid, 0) } == 0;
    exists || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

fn older_than_stale_limit(modified: Option<std::time::SystemTime>) -> bool {
    modified
        .and_then(|m| m.elapsed().ok())
        .is_none_or(|age| age.as_secs() >= crate::config::cache::STALE_TEMP_SECS)
}

/// Whether a temp leftover can go at startup: never while its owner pid runs, else by pid death or age.
pub fn leftover_is_stale(name: &str, modified: Option<std::time::SystemTime>) -> bool {
    let Some(pid) = owner_pid(name) else {
        return older_than_stale_limit(modified);
    };
    if pid == std::process::id() {
        return false;
    }
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let _ = modified;
        !process_alive(pid)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        older_than_stale_limit(modified)
    }
}

/// Remove the entries directly inside `dir` that `is_stale(name, metadata)` accepts, never following a link.
pub fn sweep_dir(
    module: &str,
    dir: &Path,
    is_stale: impl Fn(&str, &std::fs::Metadata) -> bool,
) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = path.symlink_metadata() else {
            continue;
        };
        if !is_stale(&entry.file_name().to_string_lossy(), &meta) {
            continue;
        }
        let result = if meta.is_dir() {
            std::fs::remove_dir_all(&path)
        } else {
            std::fs::remove_file(&path)
        };
        match result {
            Ok(()) => {
                removed += 1;
                log_info!(module, "Removed leftover {}", path.display());
            }
            Err(e) => log_warn!(
                module,
                "Failed to remove leftover {}: {}",
                path.display(),
                e
            ),
        }
    }
    removed
}

/// A fresh, created directory under the system temp dir for one test.
#[cfg(test)]
pub fn test_scratch_dir(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("armbian-imager-{tag}-{}", unique_suffix()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_compression_ext() {
        assert_eq!(strip_compression_ext("image.img.xz"), "image.img");
        assert_eq!(strip_compression_ext("image.img.gz"), "image.img");
        assert_eq!(strip_compression_ext("image.img.bz2"), "image.img");
        assert_eq!(strip_compression_ext("image.img.zst"), "image.img");
        assert_eq!(strip_compression_ext("image.img"), "image.img");
        assert_eq!(strip_compression_ext("no-extension"), "no-extension");
    }

    #[test]
    fn path_in_cache_is_canonical_and_contained() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        std::fs::create_dir_all(cache.join("sub")).unwrap();
        let inside = validate_path_in_cache(&cache.join("sub/../sub"), &cache).unwrap();
        assert_eq!(inside, cache.canonicalize().unwrap().join("sub"));
        assert!(validate_path_in_cache(&cache.join(".."), &cache).is_err());
        assert!(validate_path_in_cache(&cache.join("missing"), &cache).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn path_in_cache_refuses_a_symlink_out() {
        let root = tempfile::tempdir().unwrap();
        let cache = root.path().join("cache");
        let outside = root.path().join("outside");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::os::unix::fs::symlink(&outside, cache.join("link")).unwrap();
        assert!(validate_path_in_cache(&cache.join("link"), &cache).is_err());
    }

    #[test]
    fn sweep_dir_removes_only_what_the_predicate_accepts() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("work");
        std::fs::create_dir_all(dir.join("nested").join("deep")).unwrap();
        std::fs::write(dir.join("nested").join("deep").join("f"), b"x").unwrap();
        std::fs::write(dir.join("stale.img"), b"x").unwrap();
        std::fs::write(dir.join("keep.img"), b"x").unwrap();
        let outside = root.path().join("stale.img");
        std::fs::write(&outside, b"x").unwrap();

        let removed = sweep_dir("test", &dir, |name, _| name != "keep.img");
        assert_eq!(removed, 2);
        assert!(!dir.join("stale.img").exists());
        assert!(!dir.join("nested").exists());
        assert!(dir.join("keep.img").exists());
        assert!(outside.exists(), "nothing outside the dir");
        assert_eq!(
            sweep_dir("test", &root.path().join("missing"), |_, _| true),
            0
        );
    }

    #[cfg(unix)]
    #[test]
    fn sweep_dir_removes_a_link_but_not_its_target() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("work");
        let target = root.path().join("target");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("f"), b"x").unwrap();
        std::os::unix::fs::symlink(&target, dir.join("link")).unwrap();

        assert_eq!(sweep_dir("test", &dir, |_, _| true), 1);
        assert!(!dir.join("link").exists());
        assert!(target.join("f").exists());
    }

    #[test]
    fn leftover_without_a_pid_goes_only_once_old() {
        let now = std::time::SystemTime::now();
        let old = now - std::time::Duration::from_secs(crate::config::cache::STALE_TEMP_SECS + 1);
        assert!(!leftover_is_stale("qdl-extract", Some(now)));
        assert!(leftover_is_stale("qdl-extract", Some(old)));
        assert!(leftover_is_stale("qdl-extract", None));
        let ours = format!("{}.1.img", std::process::id());
        assert!(!leftover_is_stale(&ours, Some(old)));
    }

    #[test]
    fn a_recent_qdl_extraction_survives_the_startup_sweep() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("qdl-temp");
        let live = dir.join("qdl-extract");
        let crashed = dir.join("leftover");
        std::fs::create_dir_all(&live).unwrap();
        std::fs::create_dir_all(&crashed).unwrap();
        filetime::set_file_mtime(&crashed, filetime::FileTime::zero()).unwrap();

        let removed = sweep_dir("test", &dir, |name, meta| {
            leftover_is_stale(name, meta.modified().ok())
        });
        assert_eq!(removed, 1);
        assert!(live.exists(), "another instance may be flashing from it");
        assert!(!crashed.exists());
    }

    #[test]
    fn image_size_reads_the_length() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("a.img");
        std::fs::write(&file, [0u8; 300]).unwrap();
        assert_eq!(image_size(&file), Ok(300));
        assert!(image_size(&dir.path().join("missing"))
            .unwrap_err()
            .starts_with("Failed to get image size"));
    }
}
