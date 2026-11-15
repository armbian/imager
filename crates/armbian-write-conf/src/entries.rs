// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Daniele Briguglio, superkali@armbian.com

//! Write several directories and files with explicit mode and owner in one call, refusing links and odd types.

use std::collections::HashSet;
use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::{Arc, Mutex};

use armbian_ext4fs::{Errno, Ext4, InodeFileType};

use crate::{validate, PartDev, WriteConfError};

const ROOT_INODE: u32 = 2;
const EXT4_INODE_FLAG_INDEX: u32 = 0x1000;
const PERMISSION_BITS: u16 = 0o7777;

/// What an [`Entry`] creates or replaces.
#[derive(Clone, Copy)]
pub enum EntryKind<'a> {
    Dir,
    File(&'a [u8]),
}

/// One directory or file to write, with the mode (permission bits only) and owner it must end up with.
#[derive(Clone, Copy)]
pub struct Entry<'a> {
    pub path: &'a str,
    pub kind: EntryKind<'a>,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
}

impl<'a> Entry<'a> {
    pub fn dir(path: &'a str, mode: u16, uid: u32, gid: u32) -> Self {
        Entry {
            path,
            kind: EntryKind::Dir,
            mode,
            uid,
            gid,
        }
    }

    pub fn file(path: &'a str, content: &'a [u8], mode: u16, uid: u32, gid: u32) -> Self {
        Entry {
            path,
            kind: EntryKind::File(content),
            mode,
            uid,
            gid,
        }
    }

    fn is_dir(&self) -> bool {
        matches!(self.kind, EntryKind::Dir)
    }
}

// File contents can carry passwords: show only their length.
impl fmt::Debug for Entry<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct("Entry");
        s.field("path", &self.path);
        match self.kind {
            EntryKind::Dir => s.field("kind", &"dir"),
            EntryKind::File(content) => s.field("file_len", &content.len()),
        };
        s.field("mode", &format_args!("{:o}", self.mode))
            .field("uid", &self.uid)
            .field("gid", &self.gid)
            .finish()
    }
}

/// Outcome of a successful [`crate::write_entries_into_image`] call.
#[derive(Debug, Clone)]
pub struct WriteEntriesReport {
    pub scheme: &'static str,
    pub partition_offset: u64,
    pub partition_len: u64,
    pub entries: usize,
    /// File entries that replaced an existing file
    pub replaced: Vec<String>,
    pub validated: bool,
}

fn unsafe_target(path: &str, why: &str) -> WriteConfError {
    WriteConfError::UnsafeTarget(format!("{path}: {why}"))
}

/// The components of an absolute path with no empty, `.` or `..` part.
pub(crate) fn components(path: &str) -> Result<Vec<&str>, WriteConfError> {
    let rest = path
        .strip_prefix('/')
        .ok_or_else(|| unsafe_target(path, "not an absolute path"))?;
    let parts: Vec<&str> = rest.split('/').collect();
    if rest.is_empty()
        || parts
            .iter()
            .any(|p| p.is_empty() || *p == "." || *p == ".." || p.contains('\0'))
    {
        return Err(unsafe_target(path, "malformed path"));
    }
    Ok(parts)
}

pub(crate) fn join(parts: &[&str]) -> String {
    format!("/{}", parts.join("/"))
}

/// Static checks: well-formed paths, no duplicates, permission bits only.
fn check_list(entries: &[Entry]) -> Result<(), WriteConfError> {
    let mut seen = HashSet::new();
    for entry in entries {
        components(entry.path)?;
        if entry.mode & !PERMISSION_BITS != 0 {
            return Err(unsafe_target(
                entry.path,
                "mode carries more than permission bits",
            ));
        }
        if !seen.insert(entry.path) {
            return Err(unsafe_target(entry.path, "listed twice"));
        }
    }
    Ok(())
}

/// Inode number of `parts` without following links or creating anything; None when the last part is absent.
fn lookup(fs: &Ext4, path: &str, parts: &[&str]) -> Result<Option<u32>, WriteConfError> {
    let mut ino = ROOT_INODE;
    for (i, part) in parts.iter().enumerate() {
        if !fs.get_inode_ref(ino).inode.is_dir() {
            return Err(unsafe_target(path, "a path part is not a directory"));
        }
        match fs.fuse_lookup(ino as u64, part) {
            Ok(attr) => ino = attr.ino as u32,
            Err(e) if e.error() == Errno::ENOENT && i + 1 == parts.len() => return Ok(None),
            Err(e) if e.error() == Errno::ENOENT => {
                return Err(unsafe_target(path, "a parent directory is missing"))
            }
            Err(e) => return Err(WriteConfError::Ext4(format!("lookup {path}: {e:?}"))),
        }
    }
    Ok(Some(ino))
}

/// [`lookup`] that also reads a missing directory higher up as absent.
fn lookup_absent_ok(fs: &Ext4, path: &str, parts: &[&str]) -> Result<Option<u32>, WriteConfError> {
    match lookup(fs, path, parts) {
        Err(WriteConfError::UnsafeTarget(m)) if m.ends_with("a parent directory is missing") => {
            Ok(None)
        }
        other => other,
    }
}

/// The same refusals as the read-only precheck, through the write layer, before its first write; returns the file
/// entries that replace an existing file.
fn precheck_writer(fs: &Ext4, entries: &[Entry]) -> Result<Vec<String>, WriteConfError> {
    let mut planned: HashSet<String> = HashSet::new();
    let mut replaced = Vec::new();
    for entry in entries {
        let parts = components(entry.path)?;
        let parent = &parts[..parts.len() - 1];
        let parent_path = join(parent);
        // A listed parent may already exist, and then its target must pass the same checks.
        let parent_ino = if parent.is_empty() {
            Some(ROOT_INODE)
        } else {
            match lookup_absent_ok(fs, entry.path, parent)? {
                Some(ino) => Some(ino),
                None if planned.contains(&parent_path) => None,
                None => return Err(unsafe_target(entry.path, "a parent directory is missing")),
            }
        };
        if let Some(ino) = parent_ino {
            let parent_inode = fs.get_inode_ref(ino).inode;
            if !parent_inode.is_dir() {
                return Err(unsafe_target(entry.path, "a path part is not a directory"));
            }
            match lookup(fs, entry.path, &parts)? {
                Some(target) => {
                    let inode = fs.get_inode_ref(target).inode;
                    let fits = if entry.is_dir() {
                        inode.is_dir()
                    } else {
                        inode.is_file() && inode.links_count() == 1
                    };
                    if !fits {
                        return Err(unsafe_target(
                            entry.path,
                            "exists as something other than a plain directory or single-link file",
                        ));
                    }
                    if !entry.is_dir() {
                        replaced.push(entry.path.to_string());
                    }
                }
                None if parent_inode.flags() & EXT4_INODE_FLAG_INDEX != 0 => {
                    return Err(unsafe_target(
                        entry.path,
                        "the parent directory is hash-indexed",
                    ));
                }
                None => {}
            }
        }
        if entry.is_dir() {
            planned.insert(entry.path.to_string());
        }
    }
    Ok(replaced)
}

fn set_owner_and_mode(fs: &Ext4, ino: u32, file_type: InodeFileType, entry: &Entry) {
    let mut inode_ref = fs.get_inode_ref(ino);
    inode_ref.inode.set_mode(file_type.bits() | entry.mode);
    inode_ref.inode.set_uid(entry.uid as u16);
    inode_ref.inode.osd2.l_i_uid_high = (entry.uid >> 16) as u16;
    inode_ref.inode.set_gid(entry.gid as u16);
    inode_ref.inode.osd2.l_i_gid_high = (entry.gid >> 16) as u16;
    fs.write_back_inode(&mut inode_ref);
}

fn write_one(fs: &Ext4, entry: &Entry) -> Result<(), WriteConfError> {
    let parts = components(entry.path)?;
    let ext4 = |what: &str, e: &dyn fmt::Debug| {
        WriteConfError::Ext4(format!("{what} {}: {e:?}", entry.path))
    };
    match entry.kind {
        EntryKind::Dir => {
            let ino = match lookup(fs, entry.path, &parts)? {
                Some(ino) => ino,
                None => {
                    let parent =
                        lookup(fs, entry.path, &parts[..parts.len() - 1])?.ok_or_else(|| {
                            unsafe_target(entry.path, "a parent directory is missing")
                        })?;
                    let name = parts[parts.len() - 1];
                    fs.create(parent, name, InodeFileType::S_IFDIR.bits())
                        .map_err(|e| ext4("mkdir", &e))?
                        .inode_num
                }
            };
            set_owner_and_mode(fs, ino, InodeFileType::S_IFDIR, entry);
        }
        EntryKind::File(content) => {
            let ino = fs
                .ext4_file_open(entry.path, "w+")
                .map_err(|e| ext4("open", &e))?;
            fs.ext4_file_write(ino as u64, 0, content)
                .map_err(|e| ext4("write", &e))?;
            set_owner_and_mode(fs, ino, InodeFileType::S_IFREG, entry);
        }
    }
    Ok(())
}

/// Precheck read-only, write every entry in order, then validate; a refusal before the write leaves the image untouched.
pub(crate) fn write_entries_at(
    image_path: &Path,
    base: u64,
    entries: &[Entry],
) -> Result<Vec<String>, WriteConfError> {
    check_list(entries)?;
    validate::precheck_entries(image_path, base, entries)?;

    let file = OpenOptions::new().read(true).write(true).open(image_path)?;
    let dev = Arc::new(PartDev {
        file: Mutex::new(file),
        base,
    });
    let fs = Ext4::open(dev.clone());
    let replaced = precheck_writer(&fs, entries)?;
    for entry in entries {
        write_one(&fs, entry)?;
    }
    // No fsync: validation and the flash writer read this copy through the same page cache.
    drop(fs);
    drop(dev);

    validate::validate_entries(image_path, base, entries)?;
    Ok(replaced)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_must_be_absolute_and_plain() {
        assert_eq!(components("/root/.ssh").unwrap(), vec!["root", ".ssh"]);
        for bad in [
            "",
            "/",
            "root",
            "/root/",
            "//root",
            "/root/../etc",
            "/./root",
            "/a\0b",
        ] {
            assert!(components(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn the_list_refuses_duplicates_and_type_bits() {
        let dup = [Entry::dir("/a", 0o700, 0, 0), Entry::dir("/a", 0o700, 0, 0)];
        assert!(check_list(&dup).is_err());
        let typed = [Entry::file("/a", b"x", 0o100600, 0, 0)];
        assert!(check_list(&typed).is_err());
        assert!(check_list(&[Entry::file("/a", b"x", 0o600, 0, 0)]).is_ok());
    }

    #[test]
    fn debug_never_shows_content() {
        let shown = format!("{:?}", Entry::file("/p", b"SECRET", 0o600, 0, 0));
        assert!(!shown.contains("SECRET"), "{shown}");
        assert!(shown.contains("file_len: 6"), "{shown}");
    }
}
