// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Read-only ext4 validation after a write, via ext4-view, which verifies the metadata checksums it reads.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use ext4_view::{Ext4 as Ext4Ro, Ext4Error, Ext4Read};

use crate::entries::{components, join, Entry, EntryKind};
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

fn load(image_path: &Path, base: u64) -> Result<Ext4Ro, WriteConfError> {
    let file = File::open(image_path)?;
    Ext4Ro::load(Box::new(PartReader { file, base }))
        .map_err(|e| WriteConfError::Ext4(format!("ext4-view load failed: {e}")))
}

/// Read-only refusal of any entry whose path crosses a link, a non-directory or a missing parent the list does not
/// create, or whose target exists with the wrong type; nothing is ever opened for writing here.
pub fn precheck_entries(
    image_path: &Path,
    base: u64,
    entries: &[Entry],
) -> Result<(), WriteConfError> {
    let fs = load(image_path, base)?;
    let unsafe_target =
        |path: &str, why: &str| WriteConfError::UnsafeTarget(format!("{path}: {why}"));
    let mut planned: Vec<String> = Vec::new();
    for entry in entries {
        let parts = components(entry.path)?;
        for depth in 1..parts.len() {
            let prefix = join(&parts[..depth]);
            match fs.symlink_metadata(prefix.as_str()) {
                Ok(md) if md.is_dir() => {}
                Ok(_) => return Err(unsafe_target(&prefix, "not a plain directory")),
                Err(Ext4Error::NotFound) if planned.contains(&prefix) => {}
                Err(Ext4Error::NotFound) => {
                    return Err(unsafe_target(
                        &prefix,
                        "missing and not created by this write",
                    ))
                }
                Err(e) => return Err(WriteConfError::Ext4(format!("lookup {prefix}: {e}"))),
            }
        }
        match fs.symlink_metadata(entry.path) {
            Ok(md) => {
                let fits = match entry.kind {
                    EntryKind::Dir => md.is_dir(),
                    EntryKind::File(_) => md.file_type().is_regular_file(),
                };
                if !fits {
                    return Err(unsafe_target(entry.path, "exists with another file type"));
                }
            }
            Err(Ext4Error::NotFound) => {}
            Err(e) => return Err(WriteConfError::Ext4(format!("lookup {}: {e}", entry.path))),
        }
        if matches!(entry.kind, EntryKind::Dir) {
            planned.push(entry.path.to_string());
        }
    }
    Ok(())
}

/// Reload read-only and check every entry's type, mode, owner and content, then walk the whole tree.
pub fn validate_entries(
    image_path: &Path,
    base: u64,
    entries: &[Entry],
) -> Result<(), WriteConfError> {
    let fs = load(image_path, base).map_err(|e| WriteConfError::ValidationFailed(e.to_string()))?;
    let failed =
        |path: &str, why: String| WriteConfError::ValidationFailed(format!("{path}: {why}"));
    for entry in entries {
        let md = fs
            .symlink_metadata(entry.path)
            .map_err(|e| failed(entry.path, format!("metadata: {e}")))?;
        match entry.kind {
            EntryKind::Dir if !md.is_dir() => {
                return Err(failed(entry.path, "not a directory".into()))
            }
            EntryKind::Dir => {}
            EntryKind::File(content) => {
                if !md.file_type().is_regular_file() || md.len() != content.len() as u64 {
                    return Err(failed(
                        entry.path,
                        format!(
                            "expected a {} byte regular file, found {} bytes",
                            content.len(),
                            md.len()
                        ),
                    ));
                }
                let got = fs
                    .read(entry.path)
                    .map_err(|e| failed(entry.path, format!("re-read: {e}")))?;
                if got != content {
                    return Err(failed(entry.path, "content mismatch".into()));
                }
            }
        }
        if md.mode() != entry.mode || md.uid() != entry.uid || md.gid() != entry.gid {
            return Err(failed(
                entry.path,
                format!(
                    "mode {:o} owner {}:{}, expected {:o} {}:{}",
                    md.mode(),
                    md.uid(),
                    md.gid(),
                    entry.mode,
                    entry.uid,
                    entry.gid
                ),
            ));
        }
    }
    walk(&fs)
}

/// Contents of the regular file at `path`, refused when it is missing, not a regular file or larger than `cap`.
pub fn read_file_capped(
    image_path: &Path,
    base: u64,
    path: &str,
    cap: u64,
) -> Result<Vec<u8>, WriteConfError> {
    let fs = load(image_path, base)?;
    let md = fs
        .symlink_metadata(path)
        .map_err(|e| WriteConfError::Ext4(format!("lookup {path}: {e}")))?;
    if !md.file_type().is_regular_file() {
        return Err(WriteConfError::Ext4(format!(
            "{path} is not a regular file"
        )));
    }
    if md.len() > cap {
        return Err(WriteConfError::Ext4(format!(
            "{path} is {} bytes, over the {cap} byte limit",
            md.len()
        )));
    }
    fs.read(path)
        .map_err(|e| WriteConfError::Ext4(format!("read {path}: {e}")))
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
