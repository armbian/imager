use bitflags::Flags;

use crate::ext4_defs::*;
use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::bitmap::*;

impl Ext4 {
    pub fn get_bgid_of_inode(&self, inode_num: u32) -> u32 {
        inode_num / self.super_block.inodes_per_group()
    }

    pub fn inode_to_bgidx(&self, inode_num: u32) -> u32 {
        inode_num % self.super_block.inodes_per_group()
    }

    /// Get inode disk position.
    pub fn inode_disk_pos(&self, inode_num: u32) -> usize {
        let super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group;
        let inode_size = super_block.inode_size as u64;
        let group = (inode_num - 1) / inodes_per_group;
        let index = (inode_num - 1) % inodes_per_group;
        let block_group =
            Ext4BlockGroup::load_new(&self.block_device, &super_block, group as usize);
        let inode_table_blk_num = block_group.get_inode_table_blk_num();

        inode_table_blk_num as usize * BLOCK_SIZE + index as usize * inode_size as usize
    }

    /// Load the inode reference from the disk.
    pub fn get_inode_ref(&self, inode_num: u32) -> Ext4InodeRef {
        let offset = self.inode_disk_pos(inode_num);

        let mut ext4block = Block::load(&self.block_device, offset);

        let inode: &mut Ext4Inode = ext4block.read_as_mut();

        Ext4InodeRef {
            inode_num,
            inode: *inode,
        }
    }

    /// write back inode with checksum
    pub fn write_back_inode(&self, inode_ref: &mut Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        // make sure self.super_block is up-to-date
        inode_ref
            .inode
            .set_inode_checksum(&self.super_block, inode_ref.inode_num);
        inode_ref
            .inode
            .sync_inode_to_disk(&self.block_device, inode_pos);
    }

    /// write back inode with checksum
    pub fn write_back_inode_without_csum(&self, inode_ref: &Ext4InodeRef) {
        let inode_pos = self.inode_disk_pos(inode_ref.inode_num);

        inode_ref
            .inode
            .sync_inode_to_disk(&self.block_device, inode_pos);
    }

    /// Get physical block id of a logical block.
    ///
    /// Params:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id
    pub fn get_pblock_idx(&self, inode_ref: &Ext4InodeRef, lblock: Ext4Lblk) -> Result<Ext4Fsblk> {
        let search_path = self.find_extent(inode_ref, lblock);
        if let Ok(path) = search_path {
            // get the last path
            let path = path.path.last().unwrap();

            // get physical block id
            let fblock = path.pblock;

            return Ok(fblock);
        }

        return_errno_with_message!(Errno::EIO, "search extent fail");
    }

    /// Allocate a new block
    pub fn allocate_new_block(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk> {
        let mut super_block = self.super_block;
        let inodes_per_group = super_block.inodes_per_group();
        let bgid = (inode_ref.inode_num - 1) / inodes_per_group;
        let index = (inode_ref.inode_num - 1) % inodes_per_group;

        // load block group
        let mut block_group =
            Ext4BlockGroup::load_new(&self.block_device, &super_block, bgid as usize);

        let block_bitmap_block = block_group.get_block_bitmap_block(&super_block);

        let mut block_bmap_raw_data = self
            .block_device
            .read_offset(block_bitmap_block as usize * BLOCK_SIZE);
        let mut data: &mut Vec<u8> = &mut block_bmap_raw_data;
        let mut rel_blk_idx = 0;

        ext4_bmap_bit_find_clr(data, index, 0x8000, &mut rel_blk_idx);
        ext4_bmap_bit_set(data, rel_blk_idx);

        block_group.set_block_group_balloc_bitmap_csum(&super_block, data);
        self.block_device
            .write_offset(block_bitmap_block as usize * BLOCK_SIZE, data);

        /* Update superblock free blocks count */
        let mut super_blk_free_blocks = super_block.free_blocks_count();
        super_blk_free_blocks -= 1;
        super_block.set_free_blocks_count(super_blk_free_blocks);
        super_block.sync_to_disk_with_csum(&self.block_device);

        /* Update inode blocks (different block size!) count */
        let mut inode_blocks = inode_ref.inode.blocks_count();
        inode_blocks += (BLOCK_SIZE / EXT4_INODE_BLOCK_SIZE) as u64;
        inode_ref.inode.set_blocks_count(inode_blocks);
        self.write_back_inode(inode_ref);

        /* Update block group free blocks count */
        let mut fb_cnt = block_group.get_free_blocks_count();
        fb_cnt -= 1;
        block_group.set_free_blocks_count(fb_cnt as u32);
        block_group.sync_to_disk_with_csum(&self.block_device, bgid as usize, &super_block);

        Ok(rel_blk_idx as Ext4Fsblk)
    }

    /// Append a new block to the inode and update the extent tree.
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// iblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk(&self, inode_ref: &mut Ext4InodeRef) -> Result<Ext4Fsblk> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block(inode_ref, None)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Append a new block to the inode and update the extent tree.From a specific bgid
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// bgid: Start bgid of free block search
    ///
    /// Returns:
    /// `Result<Ext4Fsblk>` - physical block id of the new block
    pub fn append_inode_pblk_from(
        &self,
        inode_ref: &mut Ext4InodeRef,
        start_bgid: &mut u32,
    ) -> Result<Ext4Fsblk> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        let mut newex: Ext4Extent = Ext4Extent::default();

        let new_block = self.balloc_alloc_block_from(inode_ref, start_bgid)?;

        newex.first_block = iblock;
        newex.store_pblock(new_block);
        newex.block_count = min(1, EXT_MAX_BLOCKS - iblock) as u16;

        self.insert_extent(inode_ref, &mut newex)?;

        // Update the inode size
        let mut inode_size = inode_ref.inode.size();
        inode_size += BLOCK_SIZE as u64;
        inode_ref.inode.set_size(inode_size);
        self.write_back_inode(inode_ref);

        Ok(new_block)
    }

    /// Allocate a new inode
    ///
    /// Params:
    /// inode_mode: u16 - inode mode
    ///
    /// Returns:
    /// `Result<u32>` - inode number
    pub fn alloc_inode(&self, is_dir: bool) -> Result<u32> {
        // Allocate inode
        let inode_num = self.ialloc_alloc_inode(is_dir)?;

        Ok(inode_num)
    }

    pub fn correspond_inode_mode(&self, filetype: u8) -> u16 {
        let file_type = DirEntryType::from_bits(filetype).unwrap();
        match file_type {
            DirEntryType::EXT4_DE_REG_FILE => InodeFileType::S_IFREG.bits(),
            DirEntryType::EXT4_DE_DIR => InodeFileType::S_IFDIR.bits(),
            DirEntryType::EXT4_DE_SYMLINK => InodeFileType::S_IFLNK.bits(),
            DirEntryType::EXT4_DE_CHRDEV => InodeFileType::S_IFCHR.bits(),
            DirEntryType::EXT4_DE_BLKDEV => InodeFileType::S_IFBLK.bits(),
            DirEntryType::EXT4_DE_FIFO => InodeFileType::S_IFIFO.bits(),
            DirEntryType::EXT4_DE_SOCK => InodeFileType::S_IFSOCK.bits(),
            _ => {
                // FIXME: unsupported filetype
                InodeFileType::S_IFREG.bits()
            }
        }
    }

    /// Append multiple blocks to the inode and update the extent tree.
    ///
    /// Params:
    /// inode_ref: &mut Ext4InodeRef - inode reference
    /// start_bgid: &mut u32 - start block group id for allocation
    /// block_count: usize - number of blocks to allocate
    ///
    /// Returns:
    /// `Result<Vec<Ext4Fsblk>>` - vector of physical block ids of the new blocks
    pub fn append_inode_pblk_batch(
        &self,
        inode_ref: &mut Ext4InodeRef,
        start_bgid: &mut u32,
        block_count: usize,
    ) -> Result<Vec<Ext4Fsblk>> {
        let inode_size = inode_ref.inode.size();
        let iblock = ((inode_size as usize + BLOCK_SIZE - 1) / BLOCK_SIZE) as u32;

        // Use new optimized block allocation function
        let allocated_blocks = self.balloc_alloc_block_batch(inode_ref, start_bgid, block_count)?;

        if allocated_blocks.is_empty() {
            log::warn!("[Batch Append] No blocks could be allocated");
            return Ok(Vec::new());
        }

        // Record the actual number of allocated blocks
        let actual_allocated = allocated_blocks.len();
        if actual_allocated < block_count {
            log::warn!(
                "[Batch Append] Partial allocation: {}/{} blocks",
                actual_allocated,
                block_count
            );
        }

        // Check the current state of the extent tree
        let root_header = inode_ref.inode.root_extent_header();
        log::info!(
            "[Batch Append] Current extent tree state: magic={:x}, entries={}, max={}, depth={}",
            root_header.magic,
            root_header.entries_count,
            root_header.max_entries_count,
            root_header.depth
        );

        // Find the starting logical block position
        let mut current_iblk = iblock;
        let mut last_extent_end = if root_header.entries_count > 0 {
            // Get the end position of the last extent
            let last_extent = match self.get_last_extent(inode_ref) {
                Ok(extent) => extent.first_block + extent.block_count as u32,
                Err(_) => {
                    log::warn!(
                        "[Batch Append] Could not get last extent, starting at block {}",
                        iblock
                    );
                    iblock
                }
            };
            last_extent
        } else {
            0
        };

        // Ensure new extents start after the end of the last extent
        if current_iblk < last_extent_end {
            current_iblk = last_extent_end;
        }

        // Group allocated physical blocks into contiguous segments
        let mut contiguous_segments = Vec::new();
        let mut current_segment = Vec::new();

        // Add the first block to the current segment
        if !allocated_blocks.is_empty() {
            current_segment.push(allocated_blocks[0]);
        }

        // Check for continuity starting from the second block
        for i in 1..allocated_blocks.len() {
            let prev_block = allocated_blocks[i - 1];
            let curr_block = allocated_blocks[i];

            // If the current block is contiguous with the previous block
            if curr_block == prev_block + 1 {
                current_segment.push(curr_block);
            } else {
                // If not contiguous, end the current segment and start a new one
                if !current_segment.is_empty() {
                    contiguous_segments.push(current_segment);
                    current_segment = Vec::new();
                }
                current_segment.push(curr_block);
            }
        }

        // Add the last segment
        if !current_segment.is_empty() {
            contiguous_segments.push(current_segment);
        }

        log::info!(
            "[Batch Append] Split {} allocated blocks into {} contiguous segments",
            allocated_blocks.len(),
            contiguous_segments.len()
        );

        // Define maximum extent length
        const MAX_EXTENT_LENGTH: usize = EXT_INIT_MAX_LEN as usize;

        // Create extents for each contiguous segment
        for segment in contiguous_segments {
            if segment.is_empty() {
                continue;
            }

            // If segment length exceeds maximum extent length, split
            let mut segment_start = 0;
            while segment_start < segment.len() {
                // Calculate current segment length, ensuring it doesn't exceed MAX_EXTENT_LENGTH
                let sub_segment_length =
                    core::cmp::min(MAX_EXTENT_LENGTH, segment.len() - segment_start);
                let first_physical_block = segment[segment_start];

                // Create new extent
                let mut newex = Ext4Extent::default();
                newex.first_block = current_iblk;
                newex.store_pblock(first_physical_block);
                newex.block_count = sub_segment_length as u16;

                log::info!("[Batch Append] Inserting extent: first_block={}, block_count={}, physical_block={}", 
                    current_iblk, sub_segment_length, first_physical_block);

                // Validate extent validity
                if !self.is_valid_extent(&newex, inode_ref) {
                    log::error!(
                        "[Batch Append] Invalid extent detected: first_block={}, block_count={}",
                        newex.first_block,
                        newex.block_count
                    );
                    return return_errno_with_message!(Errno::EINVAL, "Invalid extent detected");
                }

                // Insert extent
                self.insert_extent(inode_ref, &mut newex)?;

                // Update next logical block position
                current_iblk = match current_iblk.checked_add(sub_segment_length as u32) {
                    Some(v) => v,
                    None => {
                        return return_errno_with_message!(
                            Errno::EINVAL,
                            "Logical block number overflow"
                        )
                    }
                };

                // Move to next segment
                segment_start += sub_segment_length;
            }

            // Update end position of last extent
            last_extent_end = current_iblk;

            // Validate extent tree state
            let root_header = inode_ref.inode.root_extent_header();
            log::info!("[Batch Append] Updated extent tree state: magic={:x}, entries={}, max={}, depth={}", 
                root_header.magic,
                root_header.entries_count,
                root_header.max_entries_count,
                root_header.depth);
        }

        // Update inode size, ensuring it doesn't overflow
        let new_size = match inode_size.checked_add((allocated_blocks.len() * BLOCK_SIZE) as u64) {
            Some(v) => v,
            None => return return_errno_with_message!(Errno::EINVAL, "File size overflow"),
        };
        inode_ref.inode.set_size(new_size);
        self.write_back_inode(inode_ref);

        Ok(allocated_blocks)
    }

    /// Get the last extent in the extent tree
    fn get_last_extent(&self, inode_ref: &Ext4InodeRef) -> Result<Ext4Extent> {
        let root_header = inode_ref.inode.root_extent_header();
        if root_header.entries_count == 0 {
            return return_errno_with_message!(Errno::ENOENT, "No extents found");
        }

        let mut current_header = root_header;
        let mut current_block = inode_ref.inode.root_extent_block();
        let mut depth = root_header.depth;

        while depth > 0 {
            let index_block = Block::load(&self.block_device, current_block as usize * BLOCK_SIZE);
            let index_header = Ext4ExtentHeader::load_from_u8(&index_block.data[..]);
            if index_header.entries_count == 0 {
                return return_errno_with_message!(Errno::ENOENT, "Invalid extent tree");
            }

            // Get the last index entry
            let last_idx = Ext4ExtentIndex::load_from_u8(
                &index_block.data[EXT4_EXTENT_HEADER_SIZE
                    + (index_header.entries_count - 1) as usize * EXT4_EXTENT_INDEX_SIZE..],
            );
            current_block = last_idx.leaf_lo as u64 | ((last_idx.leaf_hi as u64) << 32);
            current_header = index_header;
            depth -= 1;
        }

        // Get the last extent entry
        let extent_block = Block::load(&self.block_device, current_block as usize * BLOCK_SIZE);
        let extent_header = Ext4ExtentHeader::load_from_u8(&extent_block.data[..]);
        if extent_header.entries_count == 0 {
            return return_errno_with_message!(Errno::ENOENT, "No extent entries found");
        }

        let last_extent = Ext4Extent::load_from_u8(
            &extent_block.data[EXT4_EXTENT_HEADER_SIZE
                + (extent_header.entries_count - 1) as usize * EXT4_EXTENT_SIZE..],
        );

        Ok(last_extent)
    }

    /// Validate an extent
    fn is_valid_extent(&self, extent: &Ext4Extent, inode_ref: &Ext4InodeRef) -> bool {
        // Check if the extent is within valid range
        if extent.first_block >= EXT_MAX_BLOCKS {
            log::error!(
                "[Extent Validation] Extent first block {} exceeds maximum",
                extent.first_block
            );
            return false;
        }

        // Check if the extent length is valid
        if extent.block_count == 0 || extent.block_count > EXT_INIT_MAX_LEN {
            log::error!(
                "[Extent Validation] Invalid extent length {}",
                extent.block_count
            );
            return false;
        }

        // Check if the extent would cause overflow
        if let Some(end_block) = extent.first_block.checked_add(extent.block_count as u32) {
            if end_block > EXT_MAX_BLOCKS {
                log::error!(
                    "[Extent Validation] Extent end block {} exceeds maximum",
                    end_block
                );
                return false;
            }
        } else {
            log::error!("[Extent Validation] Extent block range overflow");
            return false;
        }

        // Check if the physical block is valid
        let pblock = extent.get_pblock();
        if pblock == 0 {
            log::error!("[Extent Validation] Invalid physical block number");
            return false;
        }

        true
    }
}
