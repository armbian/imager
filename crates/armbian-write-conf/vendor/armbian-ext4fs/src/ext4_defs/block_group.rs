use crate::prelude::*;
use crate::utils::*;

use super::*;

/// Represents the structure of an Ext4 block group descriptor.
#[derive(Debug, Default, Clone, Copy)]
#[repr(C, packed)]
pub struct Ext4BlockGroup {
    pub block_bitmap_lo: u32,            // Block bitmap block
    pub inode_bitmap_lo: u32,            // Inode bitmap block
    pub inode_table_first_block_lo: u32, // Inode table block
    pub free_blocks_count_lo: u16,       // Free blocks count
    pub free_inodes_count_lo: u16,       // Free inodes count
    pub used_dirs_count_lo: u16,         // Directories count
    pub flags: u16,                      // EXT4_BG_flags (INODE_UNINIT, etc)
    pub exclude_bitmap_lo: u32,          // Snapshot exclusion bitmap
    pub block_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+bbitmap) LE
    pub inode_bitmap_csum_lo: u16,       // crc32c(s_uuid+grp_num+ibitmap) LE
    pub itable_unused_lo: u16,           // Unused inodes count
    pub checksum: u16,                   // crc16(sb_uuid+group+desc)

    pub block_bitmap_hi: u32,            // Block bitmap block MSB
    pub inode_bitmap_hi: u32,            // Inode bitmap block MSB
    pub inode_table_first_block_hi: u32, // Inode table block MSB
    pub free_blocks_count_hi: u16,       // Free blocks count MSB
    pub free_inodes_count_hi: u16,       // Free inodes count MSB
    pub used_dirs_count_hi: u16,         // Directories count MSB
    pub itable_unused_hi: u16,           // Unused inodes count MSB
    pub exclude_bitmap_hi: u32,          // Snapshot exclusion bitmap MSB
    pub block_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+bbitmap) BE
    pub inode_bitmap_csum_hi: u16,       // crc32c(s_uuid+grp_num+ibitmap) BE
    pub reserved: u32,                   // Padding
}

impl Ext4BlockGroup {
    /// Load the block group descriptor from the disk.
    pub fn load_new(
        block_device: &Arc<dyn BlockDevice>,
        super_block: &Ext4Superblock,
        block_group_idx: usize,
    ) -> Self {
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        let dsc_id = block_group_idx / dsc_cnt;
        let first_data_block = super_block.first_data_block;
        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (block_group_idx % dsc_cnt) * super_block.desc_size as usize;

        let ext4block = Block::load(block_device, block_id * BLOCK_SIZE);
        let bg: Ext4BlockGroup = ext4block.read_offset_as(offset);

        bg
    }
}

impl Ext4BlockGroup {
    /// Get the block number of the block bitmap for this block group.
    pub fn get_block_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.block_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.block_bitmap_hi as u64) << 32;
        }
        v
    }

    /// Get the block number of the inode bitmap for this block group.
    pub fn get_inode_bitmap_block(&self, s: &Ext4Superblock) -> u64 {
        let mut v = self.inode_bitmap_lo as u64;
        let desc_size = s.desc_size;
        if desc_size > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= (self.inode_bitmap_hi as u64) << 32;
        }
        v
    }

    /// Get the count of unused inodes in this block group.
    pub fn get_itable_unused(&mut self, s: &Ext4Superblock) -> u32 {
        let mut v = self.itable_unused_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.itable_unused_hi as u64) << 32) as u32;
        }
        v
    }

    /// Get the count of used directories in this block group.
    pub fn get_used_dirs_count(&self, s: &Ext4Superblock) -> u32 {
        let mut v = self.used_dirs_count_lo as u32;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            v |= ((self.used_dirs_count_hi as u64) << 32) as u32;
        }
        v
    }

    /// Set the count of used directories in this block group.
    pub fn set_used_dirs_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16; 
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// Set the count of unused inodes in this block group.
    pub fn set_itable_unused(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.itable_unused_lo = (cnt & 0xffff) as u16; 
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.itable_unused_hi = (cnt >> 16) as u16;
        }
    }

    /// Set the count of free inodes in this block group.
    pub fn set_free_inodes_count(&mut self, s: &Ext4Superblock, cnt: u32) {
        self.free_inodes_count_lo = (cnt & 0xffff) as u16;
        if s.desc_size() > EXT4_MIN_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.free_inodes_count_hi = (cnt >> 16) as u16;
        }
    }

    /// Get the count of free inodes in this block group.
    pub fn get_free_inodes_count(&self) -> u32 {
        ((self.free_inodes_count_hi as u64) << 32) as u32 | self.free_inodes_count_lo as u32
    }

    /// Get the block number of the inode table for this block group.
    pub fn get_inode_table_blk_num(&self) -> u32 {
        ((self.inode_table_first_block_hi as u64) << 32) as u32 | self.inode_table_first_block_lo
    }
}

/// sync block group to disk
impl Ext4BlockGroup {
    /// Calculate and return the checksum of the block group descriptor.
    #[allow(unused)]
    pub fn get_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) -> u16 {
        let desc_size = super_block.desc_size();

        let mut orig_checksum = 0;
        let mut checksum = 0;

        orig_checksum = self.checksum;

        // Preparation: temporarily set bg checksum to 0
        self.checksum = 0;

        // uuid checksum
        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );

        // bgid checksum
        checksum = ext4_crc32c(checksum, &bgid.to_le_bytes(), 4);

        // cast self to &[u8]
        let self_bytes =
            unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, 0x40) };

        // bg checksum
        checksum = ext4_crc32c(checksum, self_bytes, desc_size as u32);

        self.checksum = orig_checksum;

        (checksum & 0xFFFF) as u16
    }

    /// Synchronize the block group data to disk.
    pub fn sync_block_group_to_disk(
        &self,
        block_device: &Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        let dsc_cnt = BLOCK_SIZE / super_block.desc_size as usize;
        let dsc_id = bgid / dsc_cnt;
        let first_data_block = super_block.first_data_block;
        let block_id = first_data_block as usize + dsc_id + 1;
        let offset = (bgid % dsc_cnt) * super_block.desc_size as usize;

        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4BlockGroup>())
        };
        block_device.write_offset(block_id * BLOCK_SIZE + offset, data);
    }

    /// Set the checksum of the block group descriptor.
    pub fn set_block_group_checksum(&mut self, bgid: u32, super_block: &Ext4Superblock) {
        let csum = self.get_block_group_checksum(bgid, super_block);
        self.checksum = csum;
    }

    /// Synchronize the block group data to disk with checksum.
    pub fn sync_to_disk_with_csum(
        &mut self,
        block_device: &Arc<dyn BlockDevice>,
        bgid: usize,
        super_block: &Ext4Superblock,
    ) {
        self.set_block_group_checksum(bgid as u32, super_block);
        self.sync_block_group_to_disk(block_device, bgid, super_block)
    }

    /// Set the block allocation bitmap checksum for this block group.
    pub fn set_block_group_balloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = s.ext4_balloc_bitmap_csum(bitmap);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.block_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.block_bitmap_csum_hi = hi_csum as u16;
        }
    }

    /// Get the count of free blocks in this block group.
    pub fn get_free_blocks_count(&self) -> u64 {
        let mut v = self.free_blocks_count_lo as u64;
        if self.free_blocks_count_hi != 0 {
            v |= (self.free_blocks_count_hi as u64) << 32;
        }
        v
    }

    /// Set the count of free blocks in this block group.
    pub fn set_free_blocks_count(&mut self, cnt: u32) {
        self.free_blocks_count_lo = (cnt & 0xffff) as u16; 
        self.free_blocks_count_hi = (cnt >> 16) as u16;
    }


    /// Set the inode allocation bitmap checksum for this block group.
    pub fn set_block_group_ialloc_bitmap_csum(&mut self, s: &Ext4Superblock, bitmap: &[u8]) {
        let desc_size = s.desc_size();

        let csum = s.ext4_ialloc_bitmap_csum(bitmap);
        let lo_csum = (csum & 0xFFFF).to_le();
        let hi_csum = (csum >> 16).to_le();

        if (s.features_read_only & 0x400) >> 10 == 0 {
            return;
        }
        self.inode_bitmap_csum_lo = lo_csum as u16;
        if desc_size == EXT4_MAX_BLOCK_GROUP_DESCRIPTOR_SIZE {
            self.inode_bitmap_csum_hi = hi_csum as u16;
        }
    }
}
