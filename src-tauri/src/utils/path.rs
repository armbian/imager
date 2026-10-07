// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Path manipulation helpers used across the application.

use std::path::{Path, PathBuf};

use super::app_cache_dir;

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
