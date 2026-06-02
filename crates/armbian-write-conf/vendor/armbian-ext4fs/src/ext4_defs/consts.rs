use bitflags::bitflags;

pub const BLOCK_SIZE: usize = 0x1000;                // 4KB
pub const SECTORS_PER_BLOCK: usize = BLOCK_SIZE / 512;

pub type Ext4Lblk = u32;
pub type Ext4Fsblk = u64;

pub const EOK: usize = 0;

/// Inode
pub const ROOT_INODE: u32 = 2;                      // Root directory inode
pub const JOURNAL_INODE: u32 = 8;                   // Journal file inode
pub const UNDEL_DIR_INODE: u32 = 6;                 // Undelete directory inode
pub const LOST_AND_FOUND_INODE: u32 = 11;           // lost+found directory inode
pub const EXT4_INODE_MODE_FILE: usize = 0x8000;
pub const EXT4_INODE_MODE_TYPE_MASK: u16 = 0xF000;
pub const EXT4_INODE_MODE_PERM_MASK: u16 = 0x0FFF;
pub const EXT4_INODE_BLOCK_SIZE: usize = 512;
pub const EXT4_GOOD_OLD_INODE_SIZE: u16 = 128;
pub const EXT4_INODE_FLAG_EXTENTS: usize = 0x00080000; /* Inode uses extents */

/// Extent
pub const EXT_INIT_MAX_LEN: u16 = 32768;
pub const EXT_UNWRITTEN_MAX_LEN: u16 = 65535;
pub const EXT_MAX_BLOCKS: Ext4Lblk = u32::MAX;
pub const EXT4_EXTENT_MAGIC: u16 = 0xF30A;
pub const EXT4_EXTENT_HEADER_SIZE: usize = 12;
pub const EXT4_EXTENT_SIZE: usize = 12;
pub const EXT4_EXTENT_INDEX_SIZE: usize = 12;
pub const MAX_EXTENT_INDEX_COUNT: usize = 340;

/// BLock group descriptor flags.
pub const EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 32;
pub const EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE: u16 = 64;

/// SuperBlock
pub const SUPERBLOCK_OFFSET: usize = 1024;
pub const EXT4_SUPERBLOCK_OS_HURD: u32 = 1;

/// File
pub const EXT4_MAX_FILE_SIZE: u64 = 16 * 1024 * 1024 * 1024; // 16TB

/// libc file open flags
pub const O_ACCMODE: i32 = 0o0003;
pub const O_RDONLY: i32 = 0o00;
pub const O_WRONLY: i32 = 0o01;
pub const O_RDWR: i32 = 0o02;
pub const O_CREAT: i32 = 0o0100;
pub const O_EXCL: i32 = 0o0200;
pub const O_NOCTTY: i32 = 0o0400;
pub const O_TRUNC: i32 = 0o01000;
pub const O_APPEND: i32 = 0o02000;
pub const O_NONBLOCK: i32 = 0o04000;
pub const O_SYNC: i32 = 0o4010000;
pub const O_ASYNC: i32 = 0o020000;
pub const O_LARGEFILE: i32 = 0o0100000;
pub const O_DIRECTORY: i32 = 0o0200000;
pub const O_NOFOLLOW: i32 = 0o0400000;
pub const O_CLOEXEC: i32 = 0o2000000;
pub const O_DIRECT: i32 = 0o040000;
pub const O_NOATIME: i32 = 0o1000000;
pub const O_PATH: i32 = 0o10000000;
pub const O_DSYNC: i32 = 0o010000;
/// linux access syscall flags
pub const F_OK: i32 = 0;
pub const R_OK: i32 = 4;
pub const W_OK: i32 = 2;
pub const X_OK: i32 = 1;