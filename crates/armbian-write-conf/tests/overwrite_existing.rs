mod common;

use std::path::Path;

use armbian_write_conf::{write_file_into_bare_ext4_image, WriteConfReport};
use tempfile::tempdir;

const IMAGE_SIZE: u64 = 16 * 1024 * 1024;
const INITIAL: [u8; 420] = [b'x'; 420];
const REPLACEMENT: [u8; 382] = [b'y'; 382];

fn overwrite(image: &Path) -> WriteConfReport {
    std::fs::write(image, common::ext4_bytes(IMAGE_SIZE, |_| {})).unwrap();
    write_file_into_bare_ext4_image(image, "/preset", &INITIAL).unwrap();
    write_file_into_bare_ext4_image(image, "/preset", &REPLACEMENT).unwrap()
}

#[test]
fn shorter_write_replaces_existing_file() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("image.img");

    let report = overwrite(&image);

    assert_eq!(report.bytes_written, REPLACEMENT.len());
    assert!(report.validated);
    assert_eq!(common::read_back(&image, 0, "/preset"), REPLACEMENT);
}

#[test]
fn single_write_leaves_e2fsck_clean() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("image.img");
    std::fs::write(&image, common::ext4_bytes(IMAGE_SIZE, |_| {})).unwrap();

    write_file_into_bare_ext4_image(&image, "/preset", &INITIAL).unwrap();

    common::assert_e2fsck_clean(&image, 0, "preset");
}

#[test]
#[ignore = "armbian-ext4fs keeps the replaced data block marked used, so bitmap and group free count disagree"]
fn overwrite_leaves_e2fsck_clean() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("image.img");

    overwrite(&image);

    common::assert_e2fsck_clean(&image, 0, "preset");
}
