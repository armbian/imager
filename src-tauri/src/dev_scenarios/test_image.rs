// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Sparse test images with a recognisable pattern at both ends, for flashing without a download.

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::config::dev;
use crate::utils::MB;

use super::fs::owned_dir_in;

/// An unaligned image ends mid-sector to exercise the writers' padding.
pub fn test_image_len(size_mb: u64, unaligned: bool) -> Result<u64, String> {
    if size_mb == 0 || size_mb > dev::MAX_TEST_IMAGE_MB {
        return Err(format!(
            "sizeMb must be between 1 and {}",
            dev::MAX_TEST_IMAGE_MB
        ));
    }
    let tail = if unaligned {
        dev::UNALIGNED_TAIL_BYTES
    } else {
        0
    };
    Ok(size_mb * MB + tail)
}

/// Nothing from the caller reaches the path except validated numbers.
pub fn test_image_name(size_mb: u64, unaligned: bool) -> String {
    format!(
        "{}{size_mb}{}{}{}",
        dev::TEST_IMAGE_PREFIX,
        dev::TEST_IMAGE_SIZE_UNIT,
        if unaligned {
            dev::TEST_IMAGE_UNALIGNED
        } else {
            ""
        },
        dev::TEST_IMAGE_EXT
    )
}

fn pattern(offset: u64, len: u64) -> Vec<u8> {
    (offset..offset + len)
        .map(|i| ((i % 251) as u8) ^ 0x5a)
        .collect()
}

pub fn make_test_image_in(cache: &Path, size_mb: u64, unaligned: bool) -> Result<PathBuf, String> {
    let len = test_image_len(size_mb, unaligned)?;
    let dir = owned_dir_in(cache, dev::TEST_IMAGE_DIR, true)?;
    let path = dir.join(test_image_name(size_mb, unaligned));

    if let Ok(meta) = std::fs::symlink_metadata(&path) {
        if meta.file_type().is_symlink() || !meta.is_file() {
            return Err(format!("{} is not a regular file", path.display()));
        }
        std::fs::remove_file(&path)
            .map_err(|e| format!("Failed to replace {}: {e}", path.display()))?;
    }

    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| format!("Failed to create {}: {e}", path.display()))?;
    file.set_len(len)
        .map_err(|e| format!("Failed to size {}: {e}", path.display()))?;

    let head = len.min(dev::TEST_PATTERN_BYTES);
    file.write_all(&pattern(0, head))
        .map_err(|e| format!("Failed to write test image: {e}"))?;
    let tail_start = len.saturating_sub(dev::TEST_PATTERN_BYTES).max(head);
    file.seek(SeekFrom::Start(tail_start))
        .map_err(|e| format!("Failed to write test image: {e}"))?;
    file.write_all(&pattern(tail_start, len - tail_start))
        .map_err(|e| format!("Failed to write test image: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("Failed to sync test image: {e}"))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_limits() {
        assert!(test_image_len(0, false).is_err());
        assert!(test_image_len(dev::MAX_TEST_IMAGE_MB + 1, false).is_err());
        assert_eq!(
            test_image_len(dev::MAX_TEST_IMAGE_MB, false).unwrap(),
            32 * 1024 * MB
        );
        assert_eq!(test_image_len(1, true).unwrap() % dev::SIM_SECTOR_SIZE, 300);
    }

    #[test]
    fn names_carry_only_numbers() {
        assert_eq!(test_image_name(64, false), "devsim-test-64mb.img");
        assert_eq!(test_image_name(64, true), "devsim-test-64mb-unaligned.img");
    }

    #[test]
    fn writes_pattern_at_both_ends_and_replaces() {
        let cache = tempfile::tempdir().unwrap();
        let path = make_test_image_in(cache.path(), 3, true).unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(data.len() as u64, 3 * MB + dev::UNALIGNED_TAIL_BYTES);
        assert_eq!(&data[..16], &pattern(0, 16)[..]);
        let end = data.len() as u64;
        assert_eq!(&data[data.len() - 16..], &pattern(end - 16, 16)[..]);
        assert!(data[(MB as usize) + 10..(MB as usize) + 20]
            .iter()
            .all(|b| *b == 0));
        assert_eq!(make_test_image_in(cache.path(), 3, true).unwrap(), path);
    }

    #[cfg(unix)]
    #[test]
    fn refuses_a_symlink_at_the_image_path() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        let dir = owned_dir_in(cache, dev::TEST_IMAGE_DIR, true).unwrap();
        let target = cache.join("victim");
        std::fs::write(&target, b"keep").unwrap();
        std::os::unix::fs::symlink(&target, dir.join(test_image_name(1, false))).unwrap();
        assert!(make_test_image_in(cache, 1, false).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"keep");
    }
}
