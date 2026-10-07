// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Read-only ext4 validation after a write, via ext4-view, which verifies the metadata checksums it reads.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use ext4_view::{Ext4 as Ext4Ro, Ext4Error, Ext4Read};

use crate::WriteConfError;

/// ext4-view reader over a partition window of the image file.
struct PartReader {
    file: File,
    base: u64,
}

impl Ext4Read for PartReader {
    fn read(
        &mut self,
        start_byte: u64,
        dst: &mut [u8],
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        self.file.seek(SeekFrom::Start(self.base + start_byte))?;
        self.file.read_exact(dst)?;
        Ok(())
    }
}

/// Reload read-only, check the dest file matches `content`, and walk directories and inodes; errors never carry content.
pub fn validate(
    image_path: &Path,
    base: u64,
    dest_path: &str,
    content: &[u8],
) -> Result<(), WriteConfError> {
    let file = File::open(image_path)?;
    let fs = Ext4Ro::load(Box::new(PartReader { file, base }))
        .map_err(|e| WriteConfError::ValidationFailed(format!("ext4-view load failed: {e}")))?;

    let got = fs
        .read(dest_path)
        .map_err(|e| WriteConfError::ValidationFailed(format!("re-read {dest_path}: {e}")))?;
    if got != content {
        return Err(WriteConfError::ValidationFailed(format!(
            "{dest_path} content mismatch: wrote {} bytes, read {} bytes",
            content.len(),
            got.len()
        )));
    }

    let md = fs
        .metadata(dest_path)
        .map_err(|e| WriteConfError::ValidationFailed(format!("metadata {dest_path}: {e}")))?;
    if !md.file_type().is_regular_file() || md.len() != content.len() as u64 {
        return Err(WriteConfError::ValidationFailed(format!(
            "{dest_path} inode mismatch: expected a {} byte regular file, found {} bytes",
            content.len(),
            md.len()
        )));
    }

    walk(&fs)
}

/// Whether `path` is a regular file in the ext4 filesystem at `base`; the image is opened read-only.
pub fn has_regular_file(image_path: &Path, base: u64, path: &str) -> Result<bool, WriteConfError> {
    let file = File::open(image_path)?;
    let fs = Ext4Ro::load(Box::new(PartReader { file, base }))
        .map_err(|e| WriteConfError::Ext4(format!("ext4-view load failed: {e}")))?;
    match fs.metadata(path) {
        Ok(md) => Ok(md.file_type().is_regular_file()),
        Err(Ext4Error::NotFound | Ext4Error::NotADirectory) => Ok(false),
        Err(e) => Err(WriteConfError::Ext4(format!("lookup {path}: {e}"))),
    }
}

/// Read every directory and the metadata of every entry, propagating the first error.
fn walk(fs: &Ext4Ro) -> Result<(), WriteConfError> {
    let mut pending = vec!["/".to_string()];
    while let Some(path) = pending.pop() {
        let rd = fs
            .read_dir(path.as_str())
            .map_err(|e| WriteConfError::ValidationFailed(format!("read_dir {path}: {e}")))?;
        for entry in rd {
            let entry = entry
                .map_err(|e| WriteConfError::ValidationFailed(format!("entry in {path}: {e}")))?;
            let name = match entry.file_name().as_str() {
                Ok(s) => s.to_string(),
                Err(_) => continue, // non-UTF8 name: skip
            };
            if name == "." || name == ".." {
                continue;
            }
            let child = if path == "/" {
                format!("/{name}")
            } else {
                format!("{path}/{name}")
            };
            let md = entry
                .metadata()
                .map_err(|e| WriteConfError::ValidationFailed(format!("metadata {child}: {e}")))?;
            if md.is_dir() {
                pending.push(child);
            }
        }
    }
    Ok(())
}
