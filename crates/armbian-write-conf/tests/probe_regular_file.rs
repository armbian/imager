use std::io::Cursor;

use armbian_write_conf::{rootfs_has_regular_file, WriteConfError};
use mkext4::sink::VecSink;
use mkext4::{FsBuilder, Meta, Options, ROOT};
use tempfile::tempdir;

const FS_SIZE: u64 = 16 * 1024 * 1024;
const SECTOR: u64 = 512;
const PART_START_LBA: u64 = 2048;
const LINUX_FS_GUID: [u8; 16] = [
    0xAF, 0x3D, 0xC6, 0x0F, 0x83, 0x84, 0x72, 0x47, 0x8E, 0x79, 0x3D, 0x69, 0xD8, 0x47, 0x7D, 0xE4,
];
const MARKER: &str = "/usr/lib/armbian/armbian-firstlogin";
const MTIME: (i64, u32) = (1_704_067_200, 0);

enum Marker {
    File,
    Dir,
    Missing,
}

fn ext4_with(marker: Marker) -> Vec<u8> {
    let mut b = FsBuilder::new(Options::new(FS_SIZE, [0x42; 16], MTIME.0)).unwrap();
    let dir = |mode| Meta::new(mode, 0, 0, MTIME);
    let usr = b.mkdir(ROOT, "usr", dir(0o755)).unwrap();
    let lib = b.mkdir(usr, "lib", dir(0o755)).unwrap();
    let armbian = b.mkdir(lib, "armbian", dir(0o755)).unwrap();
    let body = b"#!/bin/bash\n";
    let file = match marker {
        Marker::File => Some(
            b.file(armbian, "armbian-firstlogin", dir(0o755), body.len() as u64)
                .unwrap(),
        ),
        Marker::Dir => {
            b.mkdir(armbian, "armbian-firstlogin", dir(0o755)).unwrap();
            None
        }
        Marker::Missing => None,
    };
    let layout = b.seal().unwrap();
    let mut sink = VecSink::default();
    let mut writer = layout.writer(&mut sink).unwrap();
    if let Some(file) = file {
        writer.fill(file, &mut &body[..]).unwrap();
    }
    writer.finish().unwrap();
    sink.buf.resize(FS_SIZE as usize, 0);
    sink.buf
}

/// A GPT disk with one Linux partition holding `fs`, like a raw Armbian `.img`.
fn disk_with(fs: &[u8]) -> Vec<u8> {
    let start = PART_START_LBA * SECTOR;
    let disk_len = start + fs.len() as u64 + 1024 * 1024;
    let mut cur = Cursor::new(vec![0u8; disk_len as usize]);
    let mut gpt = gptman::GPT::new_from(&mut cur, SECTOR, [0x11; 16]).unwrap();
    gpt[1] = gptman::GPTPartitionEntry {
        partition_type_guid: LINUX_FS_GUID,
        unique_partition_guid: [0x22; 16],
        starting_lba: PART_START_LBA,
        ending_lba: PART_START_LBA + fs.len() as u64 / SECTOR - 1,
        attribute_bits: 0,
        partition_name: "rootfs".into(),
    };
    gpt.write_into(&mut cur).unwrap();
    let mut bytes = cur.into_inner();
    bytes[start as usize..start as usize + fs.len()].copy_from_slice(fs);
    bytes
}

fn probe(marker: Marker) -> Result<bool, WriteConfError> {
    let dir = tempdir().unwrap();
    let image = dir.path().join("image.img");
    std::fs::write(&image, disk_with(&ext4_with(marker))).unwrap();
    let before = std::fs::read(&image).unwrap();
    let result = rootfs_has_regular_file(&image, MARKER);
    assert_eq!(
        std::fs::read(&image).unwrap(),
        before,
        "the probe never writes"
    );
    result
}

#[test]
fn finds_a_regular_marker_file() {
    assert!(probe(Marker::File).unwrap());
}

#[test]
fn a_missing_marker_is_false_not_an_error() {
    assert!(!probe(Marker::Missing).unwrap());
}

#[test]
fn a_directory_is_not_the_marker() {
    assert!(!probe(Marker::Dir).unwrap());
}

#[test]
fn a_disk_without_ext4_is_an_error() {
    let dir = tempdir().unwrap();
    let image = dir.path().join("blank.img");
    std::fs::write(&image, disk_with(&vec![0u8; FS_SIZE as usize])).unwrap();
    assert!(matches!(
        rootfs_has_regular_file(&image, MARKER),
        Err(WriteConfError::NoExt4Rootfs(_))
    ));
}
