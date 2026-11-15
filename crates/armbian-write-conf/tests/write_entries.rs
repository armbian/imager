//! Directories and files with explicit mode and owner, on real mkfs.ext4 filesystems (bare and partitioned).

mod common;

use std::path::{Path, PathBuf};

use armbian_write_conf::{
    read_bare_ext4_file_capped, read_rootfs_file_capped, write_entries_into_bare_ext4_image,
    write_entries_into_image, Entry, WriteConfError,
};
use tempfile::{tempdir, TempDir};

const FS_SIZE: u64 = 64 * 1024 * 1024;
const PARTITION_OFFSET: u64 = 1024 * 1024;
const SECTOR: u64 = 512;
const MBR_ENTRY: usize = 0x1be;
const MBR_LINUX: u8 = 0x83;

const SSH_DIR: &str = "/root/.ssh";
const AUTHORIZED_KEYS: &str = "/root/.ssh/authorized_keys";
const USER_KEYS: &str = "/root/.imager-user-authorized-keys";
const PRESET: &str = "/root/.not_logged_in_yet";
const ROOT_KEYS: &[u8] =
    b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n root\n";
const USER_KEY_LINES: &[u8] =
    b"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgv2rp6WIB7aAlusRYNNcc8UkSS4fZFJbEjajT7nI/n user\n";
const PRESET_BODY: &[u8] = b"PRESET_USER_NAME=\"tester\"\nPRESET_USER_KEY=\"file:///root/.imager-user-authorized-keys\"\nPRESET_CONNECT_WIRELESS=\"n\"\n";

fn production() -> Vec<Entry<'static>> {
    vec![
        Entry::dir(SSH_DIR, 0o700, 0, 0),
        Entry::file(AUTHORIZED_KEYS, ROOT_KEYS, 0o600, 0, 0),
        Entry::file(USER_KEYS, USER_KEY_LINES, 0o600, 0, 0),
        Entry::file(PRESET, PRESET_BODY, 0o600, 0, 0),
    ]
}

/// A bare mkfs.ext4 filesystem with a 0700 /root, then `setup` as debugfs commands; None without e2fsprogs.
fn bare(setup: &str) -> Option<(TempDir, PathBuf)> {
    let dir = tempdir().unwrap();
    let image = dir.path().join("fs.img");
    common::mkfs_ext4(&image, FS_SIZE)?;
    common::debugfs_write(
        &image,
        &format!("mkdir root\nsif root mode 040700\n{setup}"),
    )?;
    Some((dir, image))
}

/// The same filesystem inside an MBR disk at PARTITION_OFFSET.
fn partitioned(setup: &str) -> Option<(TempDir, PathBuf)> {
    let (dir, fs) = bare(setup)?;
    let rootfs = std::fs::read(&fs).unwrap();
    let mut disk = vec![0u8; PARTITION_OFFSET as usize];
    let entry = &mut disk[MBR_ENTRY..MBR_ENTRY + 16];
    entry[4] = MBR_LINUX;
    entry[8..12].copy_from_slice(&((PARTITION_OFFSET / SECTOR) as u32).to_le_bytes());
    entry[12..16].copy_from_slice(&((FS_SIZE / SECTOR) as u32).to_le_bytes());
    disk[510] = 0x55;
    disk[511] = 0xaa;
    disk.extend_from_slice(&rootfs);
    let image = dir.path().join("disk.img");
    std::fs::write(&image, disk).unwrap();
    Some((dir, image))
}

fn at_offset(image: &Path, base: u64) -> PathBuf {
    if base == 0 {
        image.to_path_buf()
    } else {
        PathBuf::from(format!("{}?offset={base}", image.display()))
    }
}

/// Mode (octal string), uid and gid as `debugfs stat` prints them.
fn stat(image: &Path, base: u64, path: &str) -> Option<(String, u32, u32)> {
    let out = common::debugfs_read(&at_offset(image, base), &format!("stat {path}"))?;
    let after = |label: &str| -> String {
        let at = out
            .find(label)
            .unwrap_or_else(|| panic!("{label} in {out}"))
            + label.len();
        out[at..].split_whitespace().next().unwrap().to_string()
    };
    Some((
        after("Mode:"),
        after("User:").parse().unwrap(),
        after("Group:").parse().unwrap(),
    ))
}

fn assert_owned(image: &Path, base: u64, path: &str, mode: &str) {
    if let Some(found) = stat(image, base, path) {
        assert_eq!(found, (mode.to_string(), 0, 0), "{path}");
    }
}

fn assert_production_landed(image: &Path, base: u64) {
    common::assert_e2fsck_clean(image, base);
    assert_owned(image, base, SSH_DIR, "0700");
    assert_owned(image, base, AUTHORIZED_KEYS, "0600");
    assert_owned(image, base, USER_KEYS, "0600");
    assert_owned(image, base, PRESET, "0600");
    assert_eq!(common::read_back(image, base, AUTHORIZED_KEYS), ROOT_KEYS);
    assert_eq!(common::read_back(image, base, USER_KEYS), USER_KEY_LINES);
    assert_eq!(common::read_back(image, base, PRESET), PRESET_BODY);
}

fn run_bare(setup: &str) {
    let Some((_dir, image)) = bare(setup) else {
        return;
    };
    let report = write_entries_into_bare_ext4_image(&image, &production()).unwrap();
    assert!(report.validated);
    assert_eq!(report.entries, 4);
    assert_production_landed(&image, 0);
}

fn run_partitioned(setup: &str) {
    let Some((_dir, image)) = partitioned(setup) else {
        return;
    };
    let report = write_entries_into_image(&image, &production()).unwrap();
    assert_eq!(report.partition_offset, PARTITION_OFFSET);
    assert!(report.validated);
    assert_production_landed(&image, PARTITION_OFFSET);
}

const SSH_DIR_PRESENT: &str =
    "mkdir root/.ssh\nsif root/.ssh mode 040755\nsif root/.ssh uid 1000\nsif root/.ssh gid 1000\n";

fn with_existing_keys_and_preset(dir: &Path) -> String {
    let stale = dir.join("stale");
    std::fs::write(&stale, vec![b'k'; 9000]).unwrap();
    format!(
        "{SSH_DIR_PRESENT}write {0} root/.ssh/authorized_keys\nsif root/.ssh/authorized_keys mode 0100644\n\
         sif root/.ssh/authorized_keys uid 1000\nwrite {0} root/.not_logged_in_yet\n\
         sif root/.not_logged_in_yet mode 0100664\n",
        stale.display()
    )
}

#[test]
fn a_missing_ssh_dir_is_created_bare() {
    run_bare("");
}

#[test]
fn a_missing_ssh_dir_is_created_partitioned() {
    run_partitioned("");
}

#[test]
fn an_existing_ssh_dir_gets_mode_and_owner_reset() {
    run_bare(SSH_DIR_PRESENT);
    run_partitioned(SSH_DIR_PRESENT);
}

#[test]
fn existing_keys_and_preset_are_replaced_and_reset() {
    let scratch = tempdir().unwrap();
    let setup = with_existing_keys_and_preset(scratch.path());
    run_bare(&setup);
    run_partitioned(&setup);

    let Some((_dir, image)) = bare(&setup) else {
        return;
    };
    let report = write_entries_into_bare_ext4_image(&image, &production()).unwrap();
    assert_eq!(report.replaced, vec![AUTHORIZED_KEYS, PRESET]);
}

#[test]
fn a_second_run_replaces_everything_again() {
    let Some((_dir, image)) = bare("") else {
        return;
    };
    write_entries_into_bare_ext4_image(&image, &production()).unwrap();
    write_entries_into_bare_ext4_image(&image, &production()).unwrap();
    assert_production_landed(&image, 0);
}

#[test]
fn high_owner_ids_round_trip() {
    let Some((_dir, image)) = bare("") else {
        return;
    };
    let entries = [Entry::file("/root/owned", b"x", 0o640, 100_000, 70_000)];
    write_entries_into_bare_ext4_image(&image, &entries).unwrap();
    common::assert_e2fsck_clean(&image, 0);
    if let Some(found) = stat(&image, 0, "/root/owned") {
        assert_eq!(found, ("0640".to_string(), 100_000, 70_000));
    }
}

fn assert_refused_untouched(setup: &str, entries: &[Entry]) {
    let Some((_dir, image)) = bare(setup) else {
        return;
    };
    let before = std::fs::read(&image).unwrap();
    let err = write_entries_into_bare_ext4_image(&image, entries).unwrap_err();
    assert!(matches!(err, WriteConfError::UnsafeTarget(_)), "{err}");
    assert!(!err.to_string().contains("PRESET"), "{err}");
    assert!(std::fs::read(&image).unwrap() == before, "image changed");
}

#[test]
fn an_ssh_dir_that_is_a_file_is_refused_untouched() {
    let scratch = tempdir().unwrap();
    let file = scratch.path().join("f");
    std::fs::write(&file, b"x").unwrap();
    assert_refused_untouched(
        &format!("write {} root/.ssh\n", file.display()),
        &production(),
    );
}

#[test]
fn an_ssh_dir_that_is_a_link_is_refused_untouched() {
    assert_refused_untouched(
        "mkdir elsewhere\nsymlink root/.ssh /elsewhere\n",
        &production(),
    );
}

#[test]
fn authorized_keys_as_a_link_is_refused_untouched() {
    assert_refused_untouched(
        &format!("{SSH_DIR_PRESENT}symlink root/.ssh/authorized_keys /etc/shadow\n"),
        &production(),
    );
}

#[test]
fn a_hard_linked_target_is_refused_untouched() {
    let scratch = tempdir().unwrap();
    let file = scratch.path().join("f");
    std::fs::write(&file, b"x").unwrap();
    let setup = format!(
        "write {} root/.not_logged_in_yet\nln root/.not_logged_in_yet root/other\nsif root/.not_logged_in_yet links_count 2\n",
        file.display()
    );
    assert_refused_untouched(&setup, &production());
}

#[test]
fn a_hard_linked_authorized_keys_in_an_existing_ssh_dir_is_refused_untouched() {
    let scratch = tempdir().unwrap();
    let file = scratch.path().join("f");
    std::fs::write(&file, b"x").unwrap();
    let setup = format!(
        "{SSH_DIR_PRESENT}write {} root/.ssh/authorized_keys
ln root/.ssh/authorized_keys root/elsewhere
         sif root/.ssh/authorized_keys links_count 2
",
        file.display()
    );
    assert_refused_untouched(&setup, &production());
}

#[test]
fn a_missing_parent_the_list_does_not_create_is_refused_untouched() {
    assert_refused_untouched(
        "",
        &[Entry::file(
            "/root/.ssh/authorized_keys",
            ROOT_KEYS,
            0o600,
            0,
            0,
        )],
    );
    assert_refused_untouched("", &[Entry::file("/home/x/f", b"x", 0o600, 0, 0)]);
}

#[test]
fn a_directory_where_a_file_goes_is_refused_untouched() {
    assert_refused_untouched("mkdir root/.not_logged_in_yet\n", &production());
}

const SMALL_GROUPS_FS_SIZE: u64 = 128 * 1024 * 1024;
// 8192-block groups and 16 inodes each, so groups past 0 stay INODE_UNINIT.
const SMALL_GROUPS_MKFS: &[&str] = &["-b", "4096", "-g", "8192", "-N", "64"];
const INODE_NOT_INIT: &str = "Inode not init";

/// The summary and flag lines `debugfs stats` prints for `group`.
fn group_stats(stats: &str, group: u32) -> (String, String) {
    let header = format!("Group {group:>2}:");
    let mut lines = stats
        .lines()
        .skip_while(|l| !l.trim_start().starts_with(&header));
    lines
        .next()
        .unwrap_or_else(|| panic!("{header} in {stats}"));
    let summary = lines.next().unwrap_or_default().trim().to_string();
    let flags = lines.next().unwrap_or_default().trim().to_string();
    (summary, flags)
}

fn free_inodes(summary: &str) -> u32 {
    summary
        .split(", ")
        .find_map(|part| part.strip_suffix(" free inodes"))
        .unwrap_or_else(|| panic!("free inodes in {summary}"))
        .parse()
        .unwrap()
}

#[test]
fn only_uninitialized_inode_groups_left_never_corrupt_the_fs() {
    let dir = tempdir().unwrap();
    let image = dir.path().join("fs.img");
    if common::mkfs_ext4_with(&image, SMALL_GROUPS_FS_SIZE, SMALL_GROUPS_MKFS).is_none() {
        return;
    }
    common::debugfs_write(&image, "mkdir root\nsif root mode 040700\n").unwrap();
    let stats = common::debugfs_read(&image, "stats").unwrap();
    let fill: String = (0..free_inodes(&group_stats(&stats, 0).0))
        .map(|i| format!("mknod fill{i} p\n"))
        .collect();
    common::debugfs_write(&image, &fill).unwrap();

    let stats = common::debugfs_read(&image, "stats").unwrap();
    assert_eq!(free_inodes(&group_stats(&stats, 0).0), 0, "{stats}");
    let (summary, flags) = group_stats(&stats, 1);
    assert!(
        free_inodes(&summary) > 0 && flags.contains(INODE_NOT_INIT),
        "{stats}"
    );

    let before = std::fs::read(&image).unwrap();
    match write_entries_into_bare_ext4_image(&image, &production()) {
        Ok(_) => assert_production_landed(&image, 0),
        Err(err) => assert!(
            std::fs::read(&image).unwrap() == before,
            "the refused write changed the image: {err}"
        ),
    }
    common::assert_e2fsck_clean(&image, 0);
}

#[test]
fn capped_reads_fail_closed() {
    let scratch = tempdir().unwrap();
    let passwd = scratch.path().join("passwd");
    std::fs::write(&passwd, b"root:x:0:0:root:/root:/bin/bash\n").unwrap();
    let setup = format!(
        "mkdir etc\nwrite {} etc/passwd\nsymlink etc/group /etc/passwd\n",
        passwd.display()
    );
    let Some((_dir, image)) = bare(&setup) else {
        return;
    };
    assert_eq!(
        read_bare_ext4_file_capped(&image, "/etc/passwd", 1024).unwrap(),
        b"root:x:0:0:root:/root:/bin/bash\n"
    );
    assert!(read_bare_ext4_file_capped(&image, "/etc/passwd", 8).is_err());
    assert!(read_bare_ext4_file_capped(&image, "/etc/group", 1024).is_err());
    assert!(read_bare_ext4_file_capped(&image, "/etc/shadow", 1024).is_err());

    let Some((_dir, disk)) = partitioned(&setup) else {
        return;
    };
    assert!(read_rootfs_file_capped(&disk, "/etc/passwd", 1024).is_ok());
}
