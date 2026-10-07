mod common;

use std::path::Path;

use armbian_write_conf::write_file_into_bare_ext4_image;
use tempfile::tempdir;

const IMAGE_SIZE: u64 = 16 * 1024 * 1024;
const MKFS_IMAGE_SIZE: u64 = 64 * 1024 * 1024;
const DEST: &str = "/preset";
const SMALL: [u8; 382] = [b'y'; 382];
const LARGE: [u8; 9000] = [b'x'; 9000];

/// Writes every payload in turn to DEST and checks each result with validate, a read-back and e2fsck.
fn write_sequence(image: &Path, payloads: &[&[u8]]) {
    for payload in payloads {
        let report = write_file_into_bare_ext4_image(image, DEST, payload).unwrap();
        assert_eq!(report.bytes_written, payload.len());
        assert!(report.validated);
        assert_eq!(common::read_back(image, 0, DEST), *payload);
        common::assert_e2fsck_clean(image, 0);
    }
}

fn on_both_filesystems(payloads: &[&[u8]]) {
    let directory = tempdir().unwrap();
    let image = directory.path().join("mkext4.img");
    std::fs::write(&image, common::ext4_bytes(IMAGE_SIZE, |_| {})).unwrap();
    write_sequence(&image, payloads);

    let image = directory.path().join("mkfs.img");
    if common::mkfs_ext4(&image, MKFS_IMAGE_SIZE).is_some() {
        write_sequence(&image, payloads);
    }
}

#[test]
fn single_write_leaves_e2fsck_clean() {
    on_both_filesystems(&[&SMALL]);
}

#[test]
fn overwrite_leaves_e2fsck_clean() {
    on_both_filesystems(&[&[b'x'; 420], &SMALL]);
}

#[test]
fn shorter_write_replaces_existing_file() {
    on_both_filesystems(&[&LARGE, &SMALL]);
}

#[test]
fn longer_write_replaces_existing_file() {
    on_both_filesystems(&[&SMALL, &LARGE]);
}

#[test]
fn repeated_overwrites_leave_e2fsck_clean() {
    on_both_filesystems(&[&SMALL, &LARGE, &SMALL, &SMALL]);
}

#[test]
fn empty_existing_file_is_filled() {
    on_both_filesystems(&[&[], &SMALL]);
}

#[test]
fn overwrite_of_fragmented_file_frees_its_extent_tree() {
    const FILLERS: usize = 60;
    const PRESET: &str = "/root/.not_logged_in_yet";
    let directory = tempdir().unwrap();
    let image = directory.path().join("fragmented.img");
    if common::mkfs_ext4(&image, MKFS_IMAGE_SIZE).is_none() {
        return;
    }
    let filler = directory.path().join("filler");
    let preset = directory.path().join("preset");
    std::fs::write(&filler, [b'f'; 4096]).unwrap();
    std::fs::write(&preset, vec![b'p'; 100 * 4096]).unwrap();

    // Freeing every other filler leaves one-block holes, so the preset spans many extents and a depth-1 tree.
    let mut commands = String::from("mkdir /root\n");
    for i in 0..FILLERS {
        commands += &format!("write {} /f{i}\n", filler.display());
    }
    for i in (0..FILLERS).step_by(2) {
        commands += &format!("rm /f{i}\n");
    }
    commands += &format!("write {} {PRESET}\n", preset.display());
    if common::debugfs_write(&image, &commands).is_none() {
        return;
    }
    common::assert_e2fsck_clean(&image, 0);

    let report = write_file_into_bare_ext4_image(&image, PRESET, &SMALL).unwrap();

    assert!(report.validated);
    common::assert_e2fsck_clean(&image, 0);
}

#[test]
fn directory_at_destination_is_refused_untouched() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("dir.img");
    if common::mkfs_ext4(&image, MKFS_IMAGE_SIZE).is_none()
        || common::debugfs_write(&image, &format!("mkdir {DEST}\n")).is_none()
    {
        return;
    }
    let before = std::fs::read(&image).unwrap();

    assert!(write_file_into_bare_ext4_image(&image, DEST, &SMALL).is_err());

    assert!(std::fs::read(&image).unwrap() == before);
}
