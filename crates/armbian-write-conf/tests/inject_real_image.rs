//! Integration test against a real Armbian RAW image at ARMBIAN_TEST_IMAGE (with a default); if absent it prints a
//! skip notice and passes so CI without the large image doesn't fail. The write always targets a temp copy.

use std::env;
use std::path::{Path, PathBuf};

use armbian_write_conf::write_file_into_image;

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
    let tmp = temp_copy(src);
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

    let _ = std::fs::remove_file(&tmp);
}

/// Copy `src` into the OS temp dir with a unique name; panics on failure.
fn temp_copy(src: &Path) -> PathBuf {
    let mut dst = env::temp_dir();
    let pid = std::process::id();
    dst.push(format!("awc_test_{pid}.img"));
    std::fs::copy(src, &dst).expect("copy image to temp");
    dst
}
