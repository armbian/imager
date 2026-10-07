mod common;

use std::path::Path;

use tempfile::tempdir;

const IMAGE_SIZE: u64 = 64 * 1024 * 1024;
const DEST: &str = "/preset";
const CONTENT: &[u8] = b"PRESET_USER_SHELL=\"bash\"\n";

/// The write must fail with `expected` in its message and leave every byte of the image as it was.
fn assert_refused_untouched(image: &Path, expected: &str) {
    let before = std::fs::read(image).unwrap();

    let error = common::bounded_write(image, DEST, CONTENT).unwrap_err();

    assert!(error.to_string().contains(expected), "{error}");
    assert!(std::fs::read(image).unwrap() == before);
}

#[test]
fn small_block_size_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("1k.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "1024"]).is_some() {
        assert_refused_untouched(&image, "block size 1024");
    }
}

#[test]
fn missing_metadata_csum_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("nocsum.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "4096", "-O", "^metadata_csum"]).is_some()
    {
        assert_refused_untouched(&image, "metadata_csum");
    }
}

#[test]
fn missing_64bit_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("no64bit.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "4096", "-O", "^64bit"]).is_some() {
        assert_refused_untouched(&image, "without the 64bit feature");
    }
}

#[test]
fn bigalloc_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("bigalloc.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "4096", "-O", "bigalloc"]).is_some() {
        assert_refused_untouched(&image, "bigalloc");
    }
}

#[test]
fn inline_data_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("inline.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "4096", "-O", "inline_data"]).is_some() {
        assert_refused_untouched(&image, "inline_data");
    }
}

#[test]
fn journal_needing_recovery_is_refused() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("recover.img");
    if common::mkfs_ext4(&image, IMAGE_SIZE).is_none()
        || common::debugfs_write(&image, "feature needs_recovery\n").is_none()
    {
        return;
    }
    assert_refused_untouched(&image, "needs recovery");
}

fn free_inodes(image: &Path) -> usize {
    common::debugfs_read(image, "stats")
        .unwrap()
        .lines()
        .find_map(|line| line.trim().strip_prefix("Free inodes:"))
        .and_then(|n| n.trim().parse().ok())
        .unwrap()
}

#[test]
fn full_inode_table_is_refused_untouched() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("noinodes.img");
    if common::mkfs_ext4_with(&image, IMAGE_SIZE, &["-b", "4096", "-N", "16"]).is_none() {
        return;
    }
    let commands: String = (0..free_inodes(&image))
        .map(|i| format!("mknod f{i} p\n"))
        .collect();
    common::debugfs_write(&image, &commands).unwrap();
    assert_eq!(free_inodes(&image), 0);
    common::assert_e2fsck_clean(&image, 0);

    assert_refused_untouched(&image, "alloc inode fail");
}
