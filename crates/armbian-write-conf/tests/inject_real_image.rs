//! Integration test against a real Armbian RAW image at ARMBIAN_TEST_IMAGE (with a default); if absent it prints a
//! skip notice and passes so CI without the large image doesn't fail. The write always targets a temp copy.

mod common;

use std::env;
use std::path::{Path, PathBuf};

use armbian_write_conf::{
    read_rootfs_file_capped, write_entries_into_image, write_file_into_image, Entry,
};

const DEFAULT_IMAGE: &str = "/Users/danielebriguglio/Downloads/Armbian-unofficial_26.05.0-trunk_Nanopi-r76s_trixie_edge_7.0.10_minimal.img";
const DEST: &str = "/root/.not_logged_in_yet";
const CONTENT: &[u8] = b"PRESET_NET_CHANGE_DEFAULTS=\"1\"\n";

#[test]
fn inject_into_real_image() {
    let image = env::var("ARMBIAN_TEST_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string());
    let src = Path::new(&image);

    if !src.exists() {
        eprintln!("SKIP: test image not found at {image} (set ARMBIAN_TEST_IMAGE to run)");
        return;
    }

    // Copy to a temp file so the original image is never modified.
    let tmp = temp_copy(src, "preset");
    eprintln!("Operating on copy: {}", tmp.display());

    let report = write_file_into_image(&tmp, DEST, CONTENT)
        .unwrap_or_else(|e| panic!("write_file_into_image failed: {e}"));

    eprintln!(
        "scheme={} offset={} len={} dest={} bytes={} validated={}",
        report.scheme,
        report.partition_offset,
        report.partition_len,
        report.dest_path,
        report.bytes_written,
        report.validated
    );

    assert_eq!(report.bytes_written, CONTENT.len(), "byte count mismatch");
    assert!(report.validated, "report.validated must be true");

    // The full file sweep the library no longer runs, kept here as a slow oracle.
    let sweep = common::full_sweep(&tmp, report.partition_offset);
    eprintln!(
        "sweep: {} regular files, {} unreadable by ext4-view",
        sweep.files, sweep.unreadable
    );
    assert!(sweep.files > 0);
    assert_eq!(
        common::read_back(&tmp, report.partition_offset, DEST),
        CONTENT
    );
    common::assert_e2fsck_clean(&tmp, report.partition_offset);

    let _ = std::fs::remove_file(&tmp);
}

const KEYS: &[u8] =
    b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n test\n";
const USERS_CAP: u64 = 1024 * 1024;

#[test]
fn production_entries_into_real_image() {
    let image = env::var("ARMBIAN_TEST_IMAGE").unwrap_or_else(|_| DEFAULT_IMAGE.to_string());
    let src = Path::new(&image);
    if !src.exists() {
        eprintln!("SKIP: test image not found at {image} (set ARMBIAN_TEST_IMAGE to run)");
        return;
    }
    let passwd = read_rootfs_file_capped(src, "/etc/passwd", USERS_CAP).unwrap();
    assert!(passwd.starts_with(b"root:"));
    read_rootfs_file_capped(src, "/etc/group", USERS_CAP).unwrap();

    let tmp = temp_copy(src, "entries");
    let entries = [
        Entry::dir("/root/.ssh", 0o700, 0, 0),
        Entry::file("/root/.ssh/authorized_keys", KEYS, 0o600, 0, 0),
        Entry::file("/root/.imager-user-authorized-keys", KEYS, 0o600, 0, 0),
        Entry::file(DEST, CONTENT, 0o600, 0, 0),
    ];
    let report = write_entries_into_image(&tmp, &entries)
        .unwrap_or_else(|e| panic!("write_entries_into_image failed: {e}"));
    eprintln!(
        "scheme={} offset={} entries={} validated={}",
        report.scheme, report.partition_offset, report.entries, report.validated
    );
    assert!(report.validated);
    assert_eq!(
        common::read_back(&tmp, report.partition_offset, "/root/.ssh/authorized_keys"),
        KEYS
    );
    common::assert_e2fsck_clean(&tmp, report.partition_offset);
    let _ = std::fs::remove_file(&tmp);
}

/// Copy `src` into the OS temp dir with a unique name; panics on failure.
fn temp_copy(src: &Path, tag: &str) -> PathBuf {
    let mut dst = env::temp_dir();
    let pid = std::process::id();
    dst.push(format!("awc_test_{pid}_{tag}.img"));
    std::fs::copy(src, &dst).expect("copy image to temp");
    dst
}
