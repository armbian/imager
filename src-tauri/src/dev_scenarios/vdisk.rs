// SPDX-License-Identifier: GPL-2.0-or-later
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! File-backed virtual disks (macOS), confined to `<cache>/dev-vdisks/<id>.vdisk.img`.

use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::config::dev;
use crate::utils::{validate_path_in_cache, MB};

use super::fs::owned_dir_in;
use super::model::{validate_id, VdiskInfo};

pub fn file_name(id: &str) -> String {
    format!("{id}{}", dev::VDISK_SUFFIX)
}

/// Only a regular file named `<id><suffix>` directly in the vdisk dir.
fn existing_in(cache: &Path, id: &str) -> Result<PathBuf, String> {
    validate_id(id)?;
    let dir = owned_dir_in(cache, dev::VDISK_DIR, false)?;
    let name = file_name(id);
    let path = dir.join(&name);
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|e| format!("Virtual disk {id:?} not found: {e}"))?;
    if meta.file_type().is_symlink() || !meta.is_file() {
        return Err(format!("Virtual disk {id:?} is not a regular file"));
    }
    let canonical =
        validate_path_in_cache(&path, &dir).map_err(|e| format!("Virtual disk {id:?}: {e}"))?;
    if canonical.file_name() != Some(name.as_ref()) {
        return Err(format!(
            "Virtual disk {id:?} resolves outside {}",
            dir.display()
        ));
    }
    Ok(canonical)
}

pub fn create_in(cache: &Path, id: &str, size_mb: u64) -> Result<PathBuf, String> {
    validate_id(id)?;
    if size_mb == 0 || size_mb > dev::MAX_VDISK_MB {
        return Err(format!(
            "sizeMb must be between 1 and {}",
            dev::MAX_VDISK_MB
        ));
    }
    let path = owned_dir_in(cache, dev::VDISK_DIR, true)?.join(file_name(id));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|e| format!("Failed to create virtual disk {id:?}: {e}"))?;
    file.set_len(size_mb * MB)
        .map_err(|e| format!("Failed to size virtual disk {id:?}: {e}"))?;
    Ok(path)
}

/// Opens read-write without creating or following a link, then re-checks the opened inode.
pub fn open_in(cache: &Path, id: &str) -> Result<(File, PathBuf), String> {
    let path = existing_in(cache, id)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&path)
        .map_err(|e| format!("Failed to open virtual disk {id:?}: {e}"))?;
    let meta = file
        .metadata()
        .map_err(|e| format!("Failed to stat virtual disk {id:?}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("Virtual disk {id:?} is not a regular file"));
    }
    Ok((file, path))
}

pub fn size_in(cache: &Path, id: &str) -> Option<u64> {
    let path = existing_in(cache, id).ok()?;
    std::fs::metadata(path).ok().map(|m| m.len())
}

pub fn delete_in(cache: &Path, id: &str) -> Result<(), String> {
    let path = existing_in(cache, id)?;
    let meta = std::fs::symlink_metadata(&path)
        .map_err(|e| format!("Failed to stat virtual disk {id:?}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("Virtual disk {id:?} is not a regular file"));
    }
    std::fs::remove_file(&path).map_err(|e| format!("Failed to delete virtual disk {id:?}: {e}"))
}

pub fn list_in(cache: &Path) -> Vec<VdiskInfo> {
    let Ok(dir) = owned_dir_in(cache, dev::VDISK_DIR, false) else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut disks: Vec<VdiskInfo> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let id = name.strip_suffix(dev::VDISK_SUFFIX)?;
            let size_bytes = size_in(cache, id)?;
            Some(VdiskInfo {
                id: id.to_string(),
                size_bytes,
                path: dir.join(&name).to_string_lossy().to_string(),
            })
        })
        .collect();
    disks.sort_by(|a, b| a.id.cmp(&b.id));
    disks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_list_open_delete() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        let path = create_in(cache, "disk-1", 2).unwrap();
        assert!(path.ends_with("dev-vdisks/disk-1.vdisk.img"));
        assert_eq!(size_in(cache, "disk-1"), Some(2 * MB));
        assert!(
            create_in(cache, "disk-1", 2).is_err(),
            "overwrote an existing disk"
        );

        let listed = list_in(cache);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, "disk-1");

        let (file, opened) = open_in(cache, "disk-1").unwrap();
        assert!(file.metadata().unwrap().is_file());
        assert_eq!(opened, path.canonicalize().unwrap());

        delete_in(cache, "disk-1").unwrap();
        assert!(size_in(cache, "disk-1").is_none());
        assert!(open_in(cache, "disk-1").is_err(), "open must not create");
        assert!(delete_in(cache, "disk-1").is_err());
    }

    #[test]
    fn refuses_bad_ids_and_sizes() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        for id in ["", "../x", "A", "a/b", "a.b"] {
            assert!(create_in(cache, id, 1).is_err(), "{id:?} created");
            assert!(open_in(cache, id).is_err());
            assert!(delete_in(cache, id).is_err());
        }
        assert!(create_in(cache, "a", 0).is_err());
        assert!(create_in(cache, "a", dev::MAX_VDISK_MB + 1).is_err());
    }

    #[test]
    fn refuses_symlinks_in_place_of_a_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        let dir = owned_dir_in(cache, dev::VDISK_DIR, true).unwrap();
        let victim = cache.join("victim");
        std::fs::write(&victim, b"keep").unwrap();
        std::os::unix::fs::symlink(&victim, dir.join(file_name("evil"))).unwrap();

        assert!(open_in(cache, "evil").is_err());
        assert!(delete_in(cache, "evil").is_err());
        assert!(size_in(cache, "evil").is_none());
        assert!(create_in(cache, "evil", 1).is_err());
        assert!(list_in(cache).is_empty());
        assert_eq!(std::fs::read(&victim).unwrap(), b"keep");
    }

    #[test]
    fn refuses_a_directory_in_place_of_a_disk() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        let dir = owned_dir_in(cache, dev::VDISK_DIR, true).unwrap();
        std::fs::create_dir(dir.join(file_name("d"))).unwrap();
        assert!(open_in(cache, "d").is_err());
        assert!(delete_in(cache, "d").is_err());
    }

    #[test]
    fn refuses_a_symlinked_vdisk_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let cache = tmp.path();
        let outside_dir = tempfile::tempdir().unwrap();
        let outside = outside_dir.path();
        std::fs::write(outside.join(file_name("x")), vec![0u8; 1024]).unwrap();
        std::os::unix::fs::symlink(outside, cache.join(dev::VDISK_DIR)).unwrap();
        assert!(open_in(cache, "x").is_err());
        assert!(delete_in(cache, "x").is_err());
        assert!(create_in(cache, "y", 1).is_err());
        assert!(outside.join(file_name("x")).exists());
    }
}
