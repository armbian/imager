#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;

use ext4_view::{Ext4, Ext4Read};
use mkext4::sink::VecSink;
use mkext4::{FsBuilder, Options};

const SUPERBLOCK_OFFSET: usize = 1024;
const SUPERBLOCK_SIZE: usize = 1024;
const GROUP_DESCRIPTOR_SIZE_OFFSET: usize = 0xfe;
const SUPERBLOCK_CHECKSUM_OFFSET: usize = 0x3fc;
const E2FSCK_CANDIDATES: &[&str] = &[
    "e2fsck",
    "/sbin/e2fsck",
    "/usr/sbin/e2fsck",
    "/opt/homebrew/opt/e2fsprogs/sbin/e2fsck",
    "/usr/local/opt/e2fsprogs/sbin/e2fsck",
];

/// A bare mkext4 filesystem with 32-byte group descriptors, the size armbian-ext4fs writes.
pub fn ext4_bytes(size: u64, populate: impl FnOnce(&mut FsBuilder)) -> Vec<u8> {
    let mut builder = FsBuilder::new(Options::new(size, [0x42; 16], 0)).unwrap();
    populate(&mut builder);
    let layout = builder.seal().unwrap();
    let mut sink = VecSink::default();
    layout.writer(&mut sink).unwrap().finish().unwrap();
    let superblock = SUPERBLOCK_OFFSET..SUPERBLOCK_OFFSET + SUPERBLOCK_SIZE;
    let descriptor_size = SUPERBLOCK_OFFSET + GROUP_DESCRIPTOR_SIZE_OFFSET;
    sink.buf[descriptor_size..descriptor_size + 2].copy_from_slice(&32u16.to_le_bytes());
    let checksum = mkext4::csum::superblock(&sink.buf[superblock]);
    let checksum_offset = SUPERBLOCK_OFFSET + SUPERBLOCK_CHECKSUM_OFFSET;
    sink.buf[checksum_offset..checksum_offset + 4].copy_from_slice(&checksum.to_le_bytes());
    sink.buf
}

struct WindowReader {
    file: File,
    base: u64,
}

impl Ext4Read for WindowReader {
    fn read(
        &mut self,
        start_byte: u64,
        dst: &mut [u8],
    ) -> Result<(), Box<dyn core::error::Error + Send + Sync + 'static>> {
        self.file.seek(SeekFrom::Start(self.base + start_byte))?;
        self.file.read_exact(dst)?;
        Ok(())
    }
}

pub struct Sweep {
    pub files: u64,
    pub unreadable: u64,
}

/// Read every regular file of the ext4 filesystem at `base`; the image is opened read-only.
pub fn full_sweep(image: &Path, base: u64) -> Sweep {
    let file = File::open(image).unwrap();
    let fs = Ext4::load(Box::new(WindowReader { file, base })).unwrap();
    let mut sweep = Sweep {
        files: 0,
        unreadable: 0,
    };
    let mut pending = vec!["/".to_string()];
    while let Some(dir) = pending.pop() {
        for entry in fs.read_dir(dir.as_str()).unwrap() {
            let entry = entry.unwrap();
            let Ok(name) = entry.file_name().as_str().map(str::to_string) else {
                continue;
            };
            if name == "." || name == ".." {
                continue;
            }
            let child = if dir == "/" {
                format!("/{name}")
            } else {
                format!("{dir}/{name}")
            };
            let md = entry.metadata().unwrap();
            if md.is_dir() {
                pending.push(child);
            } else if md.file_type().is_regular_file() {
                sweep.files += 1;
                // ext4-view cannot resolve some legal names on real images (CA certs with '=').
                if fs.read(child.as_str()).is_err() {
                    sweep.unreadable += 1;
                }
            }
        }
    }
    sweep
}

/// Read `path` from the ext4 filesystem at `base`.
pub fn read_back(image: &Path, base: u64, path: &str) -> Vec<u8> {
    let file = File::open(image).unwrap();
    let fs = Ext4::load(Box::new(WindowReader { file, base })).unwrap();
    fs.read(path).unwrap()
}

fn e2fsck() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("E2FSCK") {
        return Some(PathBuf::from(path));
    }
    E2FSCK_CANDIDATES
        .iter()
        .map(PathBuf::from)
        .find(|candidate| {
            Command::new(candidate)
                .arg("-V")
                .output()
                .is_ok_and(|o| o.status.success())
        })
}

/// Run `e2fsck -fn` on the filesystem at `base`; None when e2fsck is not installed.
pub fn e2fsck_clean(image: &Path, base: u64) -> Option<(bool, String)> {
    let Some(bin) = e2fsck() else {
        eprintln!("SKIP e2fsck: not installed (set E2FSCK to its path)");
        return None;
    };
    let target = if base == 0 {
        image.display().to_string()
    } else {
        format!("{}?offset={base}", image.display())
    };
    let out = Command::new(bin).args(["-fn", &target]).output().unwrap();
    let log = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    Some((out.status.success(), log))
}

/// Ignores two armbian-ext4fs quirks: the file's dirent type and the free-inode total.
pub fn assert_e2fsck_clean(image: &Path, base: u64, created: &str) {
    let Some((clean, log)) = e2fsck_clean(image, base) else {
        return;
    };
    if clean {
        return;
    }
    let known = format!("Entry '{created}' in ");
    let unexpected: Vec<&str> = log
        .lines()
        .filter(|line| {
            let line = line.trim();
            !(line.is_empty()
                || line.starts_with("Pass ")
                || line.starts_with("e2fsck ")
                || line == "Fix? no"
                || line.contains("WARNING: Filesystem still has errors")
                || line.contains(" files (")
                || (line.starts_with("Free inodes count wrong (") && !line.contains("group"))
                || (line.starts_with(&known)
                    && line.ends_with("has an incorrect filetype (was 2, should be 1).")))
        })
        .collect();
    assert!(unexpected.is_empty(), "e2fsck -fn found problems:\n{log}");
}
