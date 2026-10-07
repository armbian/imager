mod common;

use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use tempfile::{tempdir, TempDir};

const IMAGE_SIZE: u64 = 64 * 1024 * 1024;
const BLOCK_SIZE: u64 = 4096;
const ENTRIES: usize = 100;
// 4 + 46 bytes makes 60-byte records, so 67 fill /root's first block and entry 68 opens the second.
const NAME_PADDING: usize = 46;
const FIRST_IN_SECOND_BLOCK: usize = 68;
const PRESET: &str = "/root/.not_logged_in_yet";
const CONTENT: &[u8] = b"PRESET_USER_SHELL=\"bash\"\n";

fn entry_name(i: usize) -> String {
    format!("f{i:03}{}", "x".repeat(NAME_PADDING))
}

/// A /root holding `entries` fifos, minus `removed`; None without e2fsprogs.
fn root_with_entries(entries: usize, removed: Option<usize>) -> Option<(TempDir, PathBuf)> {
    let directory = tempdir().unwrap();
    let image = directory.path().join("dir.img");
    common::mkfs_ext4(&image, IMAGE_SIZE)?;
    let mut commands = String::from("mkdir /root\ncd /root\n");
    for i in 1..=entries {
        commands += &format!("mknod {} p\n", entry_name(i));
    }
    if let Some(i) = removed {
        commands += &format!("rm {}\n", entry_name(i));
    }
    common::debugfs_write(&image, &commands)?;
    common::assert_e2fsck_clean(&image, 0);
    Some((directory, image))
}

/// A /root whose second directory block starts with a deleted entry (inode 0).
fn root_with_deleted_first_entry() -> Option<(TempDir, PathBuf)> {
    root_with_entries(ENTRIES, Some(FIRST_IN_SECOND_BLOCK))
}

/// Physical block of /root's second directory block, 0 when it has only one.
fn second_root_block(image: &Path) -> u64 {
    let out = common::debugfs_read(image, "bmap /root 1").unwrap();
    out.trim().parse().unwrap()
}

#[test]
fn insert_skips_deleted_entry_at_block_start() {
    let Some((_directory, image)) = root_with_deleted_first_entry() else {
        return;
    };
    let block = second_root_block(&image);
    let raw = std::fs::read(&image).unwrap();
    let start = (block * BLOCK_SIZE) as usize;
    assert_eq!(raw[start..start + 4], [0, 0, 0, 0]);

    let report = common::bounded_write(&image, PRESET, CONTENT).unwrap();

    assert!(report.validated);
    common::assert_e2fsck_clean(&image, 0);
}

#[test]
fn zero_record_length_is_refused_untouched() {
    let Some((_directory, image)) = root_with_deleted_first_entry() else {
        return;
    };
    let block = second_root_block(&image);
    let mut file = OpenOptions::new().write(true).open(&image).unwrap();
    file.seek(SeekFrom::Start(block * BLOCK_SIZE + 4)).unwrap();
    file.write_all(&0u16.to_le_bytes()).unwrap();
    drop(file);
    let before = std::fs::read(&image).unwrap();

    let error = common::bounded_write(&image, PRESET, CONTENT).unwrap_err();

    assert!(
        error.to_string().contains("Corrupt directory entry"),
        "{error}"
    );
    assert!(std::fs::read(&image).unwrap() == before);
}

#[test]
fn full_directory_block_gets_a_checksummed_new_block() {
    let Some((_directory, image)) = root_with_entries(FIRST_IN_SECOND_BLOCK - 1, None) else {
        return;
    };
    assert_eq!(second_root_block(&image), 0);

    let report = common::bounded_write(&image, PRESET, CONTENT).unwrap();

    assert!(report.validated);
    assert_ne!(second_root_block(&image), 0);
    common::assert_e2fsck_clean(&image, 0);
}

const INDEXED_ENTRIES: usize = 300;
const EXT4_INDEX_FL: u32 = 0x1000;

/// A /root that e2fsck turned into a hash-indexed directory, optionally holding the preset already.
fn indexed_root(with_preset: bool) -> Option<(TempDir, PathBuf)> {
    let directory = tempdir().unwrap();
    let image = directory.path().join("htree.img");
    common::mkfs_ext4(&image, IMAGE_SIZE)?;
    let preset = directory.path().join("preset");
    std::fs::write(&preset, b"PRESET_USER_SHELL=\"sh\"\n").unwrap();
    let mut commands = String::from("mkdir /root\ncd /root\n");
    for i in 1..=INDEXED_ENTRIES {
        commands += &format!("mknod {} p\n", entry_name(i));
    }
    if with_preset {
        commands += &format!("write {} .not_logged_in_yet\n", preset.display());
    }
    common::debugfs_write(&image, &commands)?;
    common::e2fsck_index_directories(&image)?;
    common::assert_e2fsck_clean(&image, 0);
    let stat = common::debugfs_read(&image, "stat /root")?;
    let flags = stat
        .split("Flags: 0x")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .unwrap();
    assert_ne!(flags & EXT4_INDEX_FL, 0, "e2fsck -D did not index /root");
    Some((directory, image))
}

#[test]
fn new_entry_in_indexed_directory_is_refused_untouched() {
    let Some((_directory, image)) = indexed_root(false) else {
        return;
    };
    let before = std::fs::read(&image).unwrap();

    let error = common::bounded_write(&image, PRESET, CONTENT).unwrap_err();

    assert!(error.to_string().contains("hash-indexed"), "{error}");
    assert!(std::fs::read(&image).unwrap() == before);
}

#[test]
fn existing_file_in_indexed_directory_is_overwritten() {
    let Some((_directory, image)) = indexed_root(true) else {
        return;
    };

    let report = common::bounded_write(&image, PRESET, CONTENT).unwrap();

    assert!(report.validated);
    common::assert_e2fsck_clean(&image, 0);
}
