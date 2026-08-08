use armbian_write_conf::write_file_into_bare_ext4_image;
use mkext4::sink::VecSink;
use mkext4::{FsBuilder, Options};
use tempfile::tempdir;

const IMAGE_SIZE: u64 = 16 * 1024 * 1024;
const SUPERBLOCK_OFFSET: usize = 1024;
const SUPERBLOCK_SIZE: usize = 1024;
const GROUP_DESCRIPTOR_SIZE_OFFSET: usize = 0xfe;
const SUPERBLOCK_CHECKSUM_OFFSET: usize = 0x3fc;

#[test]
fn shorter_write_replaces_existing_file() {
    let directory = tempdir().unwrap();
    let image = directory.path().join("image.img");
    let builder = FsBuilder::new(Options::new(IMAGE_SIZE, [0x42; 16], 0)).unwrap();
    let layout = builder.seal().unwrap();
    let mut sink = VecSink::default();
    layout.writer(&mut sink).unwrap().finish().unwrap();
    let superblock = SUPERBLOCK_OFFSET..SUPERBLOCK_OFFSET + SUPERBLOCK_SIZE;
    let descriptor_size = SUPERBLOCK_OFFSET + GROUP_DESCRIPTOR_SIZE_OFFSET;
    sink.buf[descriptor_size..descriptor_size + 2].copy_from_slice(&32u16.to_le_bytes());
    let checksum = mkext4::csum::superblock(&sink.buf[superblock.clone()]);
    let checksum_offset = SUPERBLOCK_OFFSET + SUPERBLOCK_CHECKSUM_OFFSET;
    sink.buf[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());
    std::fs::write(&image, sink.buf).unwrap();

    let initial = vec![b'x'; 420];
    let replacement = vec![b'y'; 382];
    write_file_into_bare_ext4_image(&image, "/preset", &initial).unwrap();
    let report = write_file_into_bare_ext4_image(&image, "/preset", &replacement).unwrap();

    assert_eq!(report.bytes_written, replacement.len());
    assert!(report.validated);
}
