mod common;

use std::path::Path;

use tempfile::tempdir;

const IMAGE_SIZE: u64 = 16 * 1024 * 1024;
const MKFS_IMAGE_SIZE: u64 = 64 * 1024 * 1024;
const DEST: &str = "/preset";
const SMALL: [u8; 382] = [b'y'; 382];
const LARGE: [u8; 9000] = [b'x'; 9000];

/// Writes every payload in turn to DEST and checks each result with validate, a read-back and e2fsck.
fn write_sequence(image: &Path, payloads: &[&[u8]]) {
    for payload in payloads {
        let report = common::bounded_write(image, DEST, payload).unwrap();
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

    let report = common::bounded_write(&image, PRESET, &SMALL).unwrap();

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

    assert!(common::bounded_write(&image, DEST, &SMALL).is_err());

    assert!(std::fs::read(&image).unwrap() == before);
}

/// Physical block ranges of `path`, parsed from debugfs `ex` lines ("... 131040 - 131103     64 ").
fn physical_ranges(image: &Path, path: &str) -> Vec<(u64, u64)> {
    common::debugfs_read(image, &format!("ex {path}"))
        .unwrap()
        .lines()
        .skip(1)
        .filter_map(|line| {
            let numbers: Vec<u64> = line
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|n| n.parse().ok())
                .collect();
            // Level, depth, entry, entries, logical start, logical end, physical start, physical end, length.
            (numbers.len() >= 9).then(|| (numbers[6], numbers[7]))
        })
        .collect()
}

fn free_blocks_per_group(image: &Path) -> Vec<u64> {
    common::debugfs_read(image, "stats")
        .unwrap()
        .lines()
        .filter_map(|line| line.trim().split_once(" free blocks,"))
        .filter_map(|(count, _)| count.trim().parse().ok())
        .collect()
}

#[test]
fn overwrite_across_block_groups_leaves_e2fsck_clean() {
    const IMAGE_SIZE: u64 = 640 * 1024 * 1024;
    const BLOCKS_PER_GROUP: u64 = 32768;
    const PRESET: &str = "/root/.not_logged_in_yet";
    const PRESET_BLOCKS: u64 = 64;
    const FILLED_GROUPS: usize = 4;
    let directory = tempdir().unwrap();
    let image = directory.path().join("multigroup.img");
    if common::mkfs_ext4(&image, IMAGE_SIZE).is_none() {
        return;
    }
    let empty = directory.path().join("empty");
    let preset = directory.path().join("preset");
    std::fs::write(&empty, []).unwrap();
    std::fs::write(&preset, vec![b'p'; (PRESET_BLOCKS * 4096) as usize]).unwrap();
    common::debugfs_write(
        &image,
        &format!("mkdir /root\nwrite {} /fill\n", empty.display()),
    )
    .unwrap();

    // Preallocating all but half a preset of groups 0-3 leaves the preset straddling groups 3 and 4.
    let free: u64 = free_blocks_per_group(&image)[..FILLED_GROUPS].iter().sum();
    let fill = free - PRESET_BLOCKS / 2;
    let commands = format!(
        "fallocate /fill 0 {}\nwrite {} {PRESET}\n",
        fill - 1,
        preset.display()
    );
    common::debugfs_write(&image, &commands).unwrap();
    let ranges = physical_ranges(&image, PRESET);
    let first_group = ranges.iter().map(|r| r.0 / BLOCKS_PER_GROUP).min().unwrap();
    let last_group = ranges.iter().map(|r| r.1 / BLOCKS_PER_GROUP).max().unwrap();
    assert!(first_group > 0 && last_group > first_group, "{ranges:?}");
    common::assert_e2fsck_clean(&image, 0);

    let longer = vec![b'z'; (PRESET_BLOCKS as usize + 16) * 4096];
    for payload in [&SMALL[..], &LARGE[..], &longer[..]] {
        let report = common::bounded_write(&image, PRESET, payload).unwrap();
        assert!(report.validated);
        assert_eq!(common::read_back(&image, 0, PRESET), payload);
        common::assert_e2fsck_clean(&image, 0);
    }
}
