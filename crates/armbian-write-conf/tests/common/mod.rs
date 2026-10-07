#![allow(dead_code)]

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use armbian_write_conf::{write_file_into_bare_ext4_image, WriteConfError, WriteConfReport};

use ext4_view::{Ext4, Ext4Read};
use mkext4::sink::VecSink;
use mkext4::{FsBuilder, Options};

const SUPERBLOCK_OFFSET: usize = 1024;
const SUPERBLOCK_SIZE: usize = 1024;
const GROUP_DESCRIPTOR_SIZE_OFFSET: usize = 0xfe;
const SUPERBLOCK_CHECKSUM_OFFSET: usize = 0x3fc;
const WRITE_TIMEOUT: Duration = Duration::from_secs(30);
const E2FSPROGS_DIRS: &[&str] = &[
    "",
    "/sbin/",
    "/usr/sbin/",
    "/opt/homebrew/opt/e2fsprogs/sbin/",
    "/usr/local/opt/e2fsprogs/sbin/",
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

/// An e2fsprogs binary from its env override (E2FSCK, MKFS_EXT4, DEBUGFS) or the usual dirs.
fn e2fsprogs_tool(name: &str) -> Option<PathBuf> {
    let var = name.to_uppercase().replace('.', "_");
    if let Some(path) = std::env::var_os(&var) {
        return Some(PathBuf::from(path));
    }
    let found = E2FSPROGS_DIRS
        .iter()
        .map(|dir| PathBuf::from(format!("{dir}{name}")))
        .find(|candidate| {
            Command::new(candidate)
                .arg("-V")
                .output()
                .is_ok_and(|o| o.status.success())
        });
    if found.is_none() {
        eprintln!("SKIP {name}: not installed (set {var} to its path)");
    }
    found
}

/// A bare filesystem made by the real mkfs.ext4 with 4K blocks; None when e2fsprogs is missing.
pub fn mkfs_ext4(image: &Path, size: u64) -> Option<()> {
    let bin = e2fsprogs_tool("mkfs.ext4")?;
    std::fs::File::create(image).unwrap().set_len(size).unwrap();
    let out = Command::new(bin)
        .args(["-q", "-F", "-b", "4096"])
        .arg(image)
        .output()
        .unwrap();
    assert!(out.status.success(), "mkfs.ext4 failed: {out:?}");
    Some(())
}

/// Run `e2fsck -fn` on the filesystem at `base`; None when e2fsck is not installed.
pub fn e2fsck_clean(image: &Path, base: u64) -> Option<(bool, String)> {
    let bin = e2fsprogs_tool("e2fsck")?;
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

/// Fails on any `e2fsck -fn` finding at `base`; skips when e2fsck is not installed.
pub fn assert_e2fsck_clean(image: &Path, base: u64) {
    if let Some((clean, log)) = e2fsck_clean(image, base) {
        assert!(clean, "e2fsck -fn found problems:\n{log}");
    }
}

/// Runs debugfs commands read-write against `image`; None when debugfs is missing.
pub fn debugfs_write(image: &Path, commands: &str) -> Option<()> {
    let bin = e2fsprogs_tool("debugfs")?;
    let script = image.with_extension("debugfs");
    std::fs::write(&script, commands).unwrap();
    let out = Command::new(bin)
        .arg("-w")
        .arg("-f")
        .arg(&script)
        .arg(image)
        .output()
        .unwrap();
    assert!(out.status.success(), "debugfs failed: {out:?}");
    Some(())
}

/// Output of one read-only debugfs request against `image`; None when debugfs is missing.
pub fn debugfs_read(image: &Path, request: &str) -> Option<String> {
    let bin = e2fsprogs_tool("debugfs")?;
    let out = Command::new(bin)
        .arg("-R")
        .arg(request)
        .arg(image)
        .output()
        .unwrap();
    assert!(out.status.success(), "debugfs failed: {out:?}");
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Runs the write on a worker thread and fails the test instead of spinning if it never returns.
pub fn bounded_write(
    image: &Path,
    dest: &str,
    content: &[u8],
) -> Result<WriteConfReport, WriteConfError> {
    let (image, dest, content) = (image.to_path_buf(), dest.to_string(), content.to_vec());
    let (done, outcome) = mpsc::channel();
    thread::spawn(move || {
        let _ = done.send(write_file_into_bare_ext4_image(&image, &dest, &content));
    });
    outcome
        .recv_timeout(WRITE_TIMEOUT)
        .expect("write did not finish in time")
}
