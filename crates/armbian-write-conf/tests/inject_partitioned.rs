mod common;

use std::path::Path;

use armbian_write_conf::{write_file_into_image, WriteConfError};
use mkext4::{Meta, ROOT};
use tempfile::tempdir;

const PARTITION_OFFSET: u64 = 1024 * 1024;
const PARTITION_SIZE: u64 = 32 * 1024 * 1024;
const SECTOR: u64 = 512;
const MBR_ENTRY: usize = 0x1be;
const MBR_LINUX: u8 = 0x83;
const DEST: &str = "/root/.not_logged_in_yet";
const PRESET: &[u8] = b"PRESET_USER_NAME=\"tester\"\nPRESET_CONNECT_WIRELESS=\"n\"\n";

/// An MBR disk with one Linux partition holding a small populated rootfs.
fn mbr_image(path: &Path) {
    let rootfs = common::ext4_bytes(PARTITION_SIZE, |b| {
        let meta = Meta::new(0o755, 0, 0, (0, 0));
        let root = b
            .mkdir(ROOT, "root", Meta::new(0o700, 0, 0, (0, 0)))
            .unwrap();
        b.file(root, ".bashrc", meta, 0).unwrap();
        let etc = b.mkdir(ROOT, "etc", meta).unwrap();
        let ssl = b.mkdir(etc, "ssl", meta).unwrap();
        b.mkdir(ssl, "certs", meta).unwrap();
        b.symlink(etc, "mtab", "/proc/self/mounts", meta).unwrap();
    });
    let mut disk = vec![0u8; (PARTITION_OFFSET + PARTITION_SIZE) as usize];
    let entry = &mut disk[MBR_ENTRY..MBR_ENTRY + 16];
    entry[4] = MBR_LINUX;
    entry[8..12].copy_from_slice(&((PARTITION_OFFSET / SECTOR) as u32).to_le_bytes());
    entry[12..16].copy_from_slice(&((PARTITION_SIZE / SECTOR) as u32).to_le_bytes());
    disk[510] = 0x55;
    disk[511] = 0xaa;
    disk[PARTITION_OFFSET as usize..].copy_from_slice(&rootfs);
    std::fs::write(path, disk).unwrap();
}

#[test]
fn preset_lands_in_a_partitioned_image() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("disk.img");
    mbr_image(&image);

    let report = write_file_into_image(&image, DEST, PRESET).unwrap();

    assert_eq!(report.scheme, "MBR");
    assert_eq!(report.partition_offset, PARTITION_OFFSET);
    assert!(report.validated);
    assert_eq!(common::read_back(&image, PARTITION_OFFSET, DEST), PRESET);
    let sweep = common::full_sweep(&image, PARTITION_OFFSET);
    assert_eq!(sweep.files, 2);
    assert_eq!(sweep.unreadable, 0);
    common::assert_e2fsck_clean(&image, PARTITION_OFFSET);
}

#[test]
fn a_second_injection_replaces_the_preset() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("disk.img");
    mbr_image(&image);
    let longer = [PRESET, PRESET].concat();

    write_file_into_image(&image, DEST, &longer).unwrap();
    write_file_into_image(&image, DEST, PRESET).unwrap();

    assert_eq!(common::read_back(&image, PARTITION_OFFSET, DEST), PRESET);
    common::assert_e2fsck_clean(&image, PARTITION_OFFSET);
}

#[test]
fn a_broken_directory_away_from_the_preset_fails_validation() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("disk.img");
    mbr_image(&image);
    let mut disk = std::fs::read(&image).unwrap();
    // The "certs" dirent in /etc/ssl: name_len 5, file type dir, then the name.
    let needle = [&[5u8, 2][..], b"certs"].concat();
    let at = disk
        .windows(needle.len())
        .position(|w| w == needle)
        .expect("certs dirent");
    disk[at + 2] ^= 0x20;
    std::fs::write(&image, disk).unwrap();

    let err = write_file_into_image(&image, DEST, PRESET).unwrap_err();
    assert!(matches!(err, WriteConfError::ValidationFailed(_)), "{err}");
    assert!(!err.to_string().contains("PRESET"), "{err}");
}
