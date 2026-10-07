// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Containment checks for the emulator's own cache subdirectories.

use std::path::{Path, PathBuf};

use crate::utils::validate_path_in_cache;

/// Refused unless a real directory (not a link) directly in the cache.
pub fn owned_dir_in(cache: &Path, name: &str, create: bool) -> Result<PathBuf, String> {
    let dir = cache.join(name);
    if create {
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    }
    let meta = std::fs::symlink_metadata(&dir)
        .map_err(|e| format!("Failed to stat {}: {e}", dir.display()))?;
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(format!("{} is not a real directory", dir.display()));
    }
    let canonical_dir =
        validate_path_in_cache(&dir, cache).map_err(|e| format!("{}: {e}", dir.display()))?;
    let canonical_cache = cache
        .canonicalize()
        .map_err(|e| format!("Failed to resolve cache directory: {e}"))?;
    if canonical_dir != canonical_cache.join(name) {
        return Err(format!(
            "{} resolves outside the cache directory",
            dir.display()
        ));
    }
    Ok(canonical_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_accepts_a_real_subdirectory() {
        let cache = tempfile::tempdir().unwrap();
        let cache = cache.path();
        let dir = owned_dir_in(cache, "sub", true).unwrap();
        assert_eq!(dir, cache.canonicalize().unwrap().join("sub"));
        assert!(owned_dir_in(cache, "sub", false).is_ok());
        assert!(owned_dir_in(cache, "missing", false).is_err());
    }

    #[test]
    fn refuses_a_file_in_place_of_the_directory() {
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(cache.path().join("sub"), b"x").unwrap();
        assert!(owned_dir_in(cache.path(), "sub", false).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlinked_directory() {
        let cache = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), cache.path().join("sub")).unwrap();
        assert!(owned_dir_in(cache.path(), "sub", false).is_err());
        assert!(owned_dir_in(cache.path(), "sub", true).is_err());
    }
}
