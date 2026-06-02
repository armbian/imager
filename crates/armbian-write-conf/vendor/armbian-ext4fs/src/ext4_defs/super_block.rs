use crate::prelude::*;
use crate::utils::*;

use super::*;
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Superblock {
    pub inodes_count: u32,         // Inodes count
    blocks_count_lo: u32,          // Blocks count
    reserved_blocks_count_lo: u32, // Reserved blocks count
    free_blocks_count_lo: u32,     // Free blocks count
    free_inodes_count: u32,        // Free inodes count
    pub first_data_block: u32,     // First data block
    log_block_size: u32,           // Block size
    log_cluster_size: u32,         // Deprecated fragment size
    blocks_per_group: u32,         // Blocks per group
    frags_per_group: u32,          // Deprecated fragments per group
    pub inodes_per_group: u32,     // Inodes per group
    mount_time: u32,               // Mount time
    write_time: u32,               // Write time
    mount_count: u16,              // Mount count
    max_mount_count: u16,          // Maximum mount count
    magic: u16,                    // Magic signature, 0xEF53
    state: u16,                    // Filesystem state
    errors: u16,                   // Behavior when errors detected
    minor_rev_level: u16,          // Minor revision level
    last_check_time: u32,          // Last check time
    check_interval: u32,           // Check interval
    pub creator_os: u32,           // Creator OS
    pub rev_level: u32,            // Revision level
    def_resuid: u16,               // Default reserved blocks uid
    def_resgid: u16,               // Default reserved blocks gid

    // Fields for EXT4_DYNAMIC_REV superblocks only
    first_inode: u32,            // First non-reserved inode
    pub inode_size: u16,         // Size of inode structure
    block_group_index: u16,      // Block group index of this superblock
    features_compatible: u32,    // Compatible feature set
    features_incompatible: u32,  // Incompatible feature set
    pub features_read_only: u32, // Read-only compatible feature set
    pub uuid: [u8; 16],          // 128-bit UUID for volume
    volume_name: [u8; 16],       // Volume name
    last_mounted: [u8; 64],      // Directory where last mounted
    algorithm_usage_bitmap: u32, // Algorithm usage bitmap

    // Performance hints. Directory preallocation only when EXT4_FEATURE_COMPAT_DIR_PREALLOC flag is on
    s_prealloc_blocks: u8,      // Number of blocks to try to preallocate
    s_prealloc_dir_blocks: u8,  // Number of blocks to preallocate for directories
    s_reserved_gdt_blocks: u16, // Number of reserved GDT entries for online growth per group

    // Journaling support - if EXT4_FEATURE_COMPAT_HAS_JOURNAL set
    journal_uuid: [u8; 16],    // UUID of journal superblock
    journal_inode_number: u32, // Inode number of journal file
    journal_dev: u32,          // Device number of journal file
    last_orphan: u32,          // Head of list of inodes to delete
    hash_seed: [u32; 4],       // HTREE hash seed
    default_hash_version: u8,  // Default hash version
    journal_backup_type: u8,
    pub desc_size: u16,        // Group descriptor size
    default_mount_opts: u32,   // Default mount options
    first_meta_bg: u32,        // First metadata block group
    mkfs_time: u32,            // Filesystem creation time
    journal_blocks: [u32; 17], // Journal node backup

    // If EXT4_FEATURE_COMPAT_64BIT set, supports 64-bit
    blocks_count_hi: u32,          // Blocks count
    reserved_blocks_count_hi: u32, // Reserved blocks count
    free_blocks_count_hi: u32,     // Free blocks count
    min_extra_isize: u16,          // All inodes have at least # bytes
    want_extra_isize: u16,         // New inodes should reserve # bytes
    flags: u32,                    // Miscellaneous flags
    raid_stride: u16,              // RAID stride
    mmp_interval: u16,             // MMP check wait seconds
    mmp_block: u64,                // Multi-mount protection block
    raid_stripe_width: u32,        // Blocks on all data disks (N * stride)
    log_groups_per_flex: u8,       // FLEX_BG group size
    checksum_type: u8,
    reserved_pad: u16,
    kbytes_written: u64,          // Written kilobytes
    snapshot_inum: u32,           // Active snapshot inode number
    snapshot_id: u32,             // Active snapshot sequence ID
    snapshot_r_blocks_count: u64, // Reserved blocks for future use of active snapshot
    snapshot_list: u32,           // Head node number of snapshot list on disk
    error_count: u32,             // Number of filesystem errors
    first_error_time: u32,        // Time of first error occurrence
    first_error_ino: u32,         // Inode number of first error occurrence
    first_error_block: u64,       // Block number of first error occurrence
    first_error_func: [u8; 32],   // Function of first error occurrence
    first_error_line: u32,        // Line number of first error occurrence
    last_error_time: u32,         // Time of last error occurrence
    last_error_ino: u32,          // Inode number of last error occurrence
    last_error_line: u32,         // Line number of last error occurrence
    last_error_block: u64,        // Block number of last error occurrence
    last_error_func: [u8; 32],    // Function of last error occurrence
    mount_opts: [u8; 64],
    usr_quota_inum: u32,       // Node for tracking user quota
    grp_quota_inum: u32,       // Node for tracking group quota
    overhead_clusters: u32,    // Overhead blocks/clusters in filesystem
    backup_bgs: [u32; 2],      // Groups with sparse_super2 superblock
    encrypt_algos: [u8; 4],    // Used encryption algorithms
    encrypt_pw_salt: [u8; 16], // Salt for string2key algorithm
    lpf_ino: u32,              // Location of lost+found node
    padding: [u32; 100],       // Padding at end of block
    checksum: u32,             // crc32c(superblock)
}

impl Ext4Superblock {
    /// Returns the size of inode structure.
    pub fn inode_size(&self) -> u16 {
        self.inode_size
    }

    /// Returns the size of inode structure.
    pub fn inode_size_file(&self, inode: &Ext4Inode) -> u64 {
        let mode = inode.mode;
        let mut v = inode.size as u64;
        if self.rev_level > 0 && (mode & EXT4_INODE_MODE_TYPE_MASK) == EXT4_INODE_MODE_FILE as u16 {
            let hi = (inode.size_hi as u64) << 32;
            v |= hi;
        }
        v
    }

    pub fn free_inodes_count(&self) -> u32 {
        self.free_inodes_count
    }

    /// Returns total number of inodes.
    pub fn total_inodes(&self) -> u32 {
        self.inodes_count
    }

    /// Returns the number of blocks in each block group.
    pub fn blocks_per_group(&self) -> u32 {
        self.blocks_per_group
    }

    /// Returns the size of block.
    pub fn block_size(&self) -> u32 {
        1024 << self.log_block_size
    }

    /// Returns the number of inodes in each block group.
    pub fn inodes_per_group(&self) -> u32 {
        self.inodes_per_group
    }

    /// Returns the first data block.
    pub fn first_data_block(&self) -> u32{
        self.first_data_block
    }

    /// Returns the number of block groups.
    pub fn block_group_count(&self) -> u32 {
        let blocks_count = (self.blocks_count_hi as u64) << 32 | self.blocks_count_lo as u64;

        let blocks_per_group = self.blocks_per_group as u64;

        let mut block_group_count = blocks_count / blocks_per_group;

        if (blocks_count % blocks_per_group) != 0 {
            block_group_count += 1;
        }

        block_group_count as u32
    }

    pub fn blocks_count(&self) -> u32 {
        ((self.blocks_count_hi.to_le() as u64) << 32) as u32 | self.blocks_count_lo
    }

    pub fn desc_size(&self) -> u16 {
        let size = self.desc_size;

        if size < EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE
        } else {
            size
        }
    }

    pub fn extra_size(&self) -> u16 {
        self.want_extra_isize
    }

    pub fn get_inodes_in_group_cnt(&self, bgid: u32) -> u32 {
        let block_group_count = self.block_group_count();
        let inodes_per_group = self.inodes_per_group;

        let total_inodes = self.inodes_count;
        if bgid < block_group_count - 1 {
            inodes_per_group
        } else {
            total_inodes - ((block_group_count - 1) * inodes_per_group)
        }
    }

    pub fn decrease_free_inodes_count(&mut self) {
        self.free_inodes_count -= 1;
    }

    pub fn free_blocks_count(&self) -> u64 {
        self.free_blocks_count_lo as u64 | ((self.free_blocks_count_hi as u64) << 32).to_le()
    }

    pub fn set_free_blocks_count(&mut self, free_blocks: u64) {
        self.free_blocks_count_lo = (free_blocks & 0xffffffff) as u32; 

        self.free_blocks_count_hi = (free_blocks >> 32) as u32;
    }

    pub fn sync_to_disk(&self, block_device: &Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(SUPERBLOCK_OFFSET, data);
    }

    pub fn sync_to_disk_with_csum(&mut self, block_device: &Arc<dyn BlockDevice>) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        let checksum = ext4_crc32c(EXT4_CRC32_INIT, data, 0x3fc);

        self.checksum = checksum;
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Superblock>())
        };
        block_device.write_offset(SUPERBLOCK_OFFSET, data);
    }

    pub fn incompat_features(&self) -> u32 {
        self.features_incompatible
    }
    
    pub fn reserved_gdt_blocks(&self) -> u16 {
        self.s_reserved_gdt_blocks
    }
}

impl Ext4Superblock {
    /// Returns the checksum of the block bitmap
    pub fn ext4_balloc_bitmap_csum(&self, bitmap: &[u8]) -> u32 {
        let mut csum = 0;
        let blocks_per_group = self.blocks_per_group;
        let uuid = self.uuid;
        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
        csum = ext4_crc32c(csum, bitmap, blocks_per_group / 8);
        csum
    }

    /// Returns the checksum of the inode bitmap
    pub fn ext4_ialloc_bitmap_csum(&self, bitmap: &[u8]) -> u32 {
        let mut csum = 0;
        let inodes_per_group = self.inodes_per_group;
        let uuid = self.uuid;
        csum = ext4_crc32c(EXT4_CRC32_INIT, &uuid, uuid.len() as u32);
        csum = ext4_crc32c(csum, bitmap, (inodes_per_group + 7) / 8);
        csum
    }
}
