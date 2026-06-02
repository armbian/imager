use crate::prelude::*;
use crate::return_errno_with_message;
use crate::ext4_defs::*;
use alloc::format;
use core::mem::size_of;
use crate::utils::crc::*;


impl Ext4 {
    /// Find an extent in the extent tree.
    ///
    /// Params:
    /// inode_ref: &Ext4InodeRef - inode reference
    /// lblock: Ext4Lblk - logical block id
    ///
    /// Returns:
    /// `Result<SearchPath>` - search path
    ///
    /// If depth > 0, search for the extent_index that corresponds to the target lblock.
    /// If depth = 0, directly search for the extent in the root node that corresponds to the target lblock.
    pub fn find_extent(&self, inode_ref: &Ext4InodeRef, lblock: Ext4Lblk) -> Result<SearchPath> {
        let mut search_path = SearchPath::new();

        // Load the root node
        let root_data: &[u8; 60] =
            unsafe { core::mem::transmute::<&[u32; 15], &[u8; 60]>(&inode_ref.inode.block) };
        let mut node = ExtentNode::load_from_data(root_data, true).unwrap();

        let mut depth = node.header.depth;

        // Traverse down the tree if depth > 0
        let mut pblock_of_node = 0;
        while depth > 0 {
            let index_pos = node.binsearch_idx(lblock);
            if let Some(pos) = index_pos {
                let index = node.get_index(pos)?;
                let next_block = index.leaf_lo;

                search_path.path.push(ExtentPathNode {
                    header: node.header,
                    index: Some(index),
                    extent: None,
                    position: pos,
                    pblock: next_block as u64,
                    pblock_of_node,
                });

                let next_block = search_path.path.last().unwrap().index.unwrap().leaf_lo;
                let mut next_data = self
                    .block_device
                    .read_offset(next_block as usize * BLOCK_SIZE);
                node = ExtentNode::load_from_data_mut(&mut next_data, false)?;
                depth -= 1;
                search_path.depth += 1;
                pblock_of_node = next_block as usize;
            } else {
                return_errno_with_message!(Errno::ENOENT, "Extentindex not found");
            }
        }

        // Handle the case where depth is 0
        if let Some((extent, pos)) = node.binsearch_extent(lblock) {
            search_path.path.push(ExtentPathNode {
                header: node.header,
                index: None,
                extent: Some(extent),
                position: pos,
                pblock: lblock as u64 - extent.get_first_block() as u64 + extent.get_pblock(),
                pblock_of_node,
            });
            search_path.maxdepth = node.header.depth;

            Ok(search_path)
        } else {
            search_path.path.push(ExtentPathNode {
                header: node.header,
                index: None,
                extent: None,
                position: 0,
                pblock: 0,
                pblock_of_node,
            });
            Ok(search_path)
        }
    }

    /// Insert an extent into the extent tree.
    pub fn insert_extent(
        &self,
        inode_ref: &mut Ext4InodeRef,
        newex: &mut Ext4Extent,
    ) -> Result<()> {
        let newex_first_block = newex.first_block;
        log::info!("[insert_extent] Starting - Inserting extent at block {}", newex_first_block);
        log::info!("[insert_extent] Current tree state: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);
        
        let mut search_path = self.find_extent(inode_ref, newex_first_block)?;
        
        let depth = search_path.depth as usize;
        let node = &search_path.path[depth]; // Get the node at the current depth

        let at_root = node.pblock_of_node == 0;
        let header = node.header;

        // Node is empty (no extents)
        if header.entries_count == 0 {
            log::info!("[insert_extent] Node is empty, inserting directly");
            self.insert_new_extent(inode_ref, &mut search_path, newex)?;
            return Ok(());
        }

        // Insert to exsiting extent
        if let Some(mut ex) = node.extent {
            let pos = node.position;
            let last_extent_pos = header.entries_count as usize - 1;

            // Try to Insert to found_ext
            // found_ext:   |<---found_ext--->|         |<---ext2--->|
            //              20              30         50          60
            // insert:      |<---found_ext---><---newex--->|         |<---ext2--->|
            //              20              30            40         50          60
            // merge:       |<---newex--->|      |<---ext2--->|
            //              20           40      50          60
            if self.can_merge(&ex, newex) {
                self.merge_extent(&search_path, &mut ex, newex)?;

                if at_root {
                    // we are at root
                    *inode_ref.inode.root_extent_mut_at(node.position) = ex;
                }
                return Ok(());
            }

            // Insert right
            // found_ext:   |<---found_ext--->|         |<---next_extent--->|
            //              10               20         30                40
            // insert:      |<---found_ext--->|<---newex---><---next_extent--->|
            //              10               20            30                40
            // merge:       |<---found_ext--->|<---newex--->|
            //              10               20            40
            if pos < last_extent_pos
                && ((ex.first_block + ex.block_count as u32) < newex.first_block)
            {
                if let Ok(next_extent) = self.get_extent_from_node(node, pos + 1) {
                    if self.can_merge(&next_extent, newex) {
                        self.merge_extent(&search_path, newex, &next_extent)?;
                        return Ok(());
                    }
                }
            }

            // Insert left
            //  found_ext:  |<---found_ext--->|         |<---ext2--->|
            //              20              30         40          50
            // insert:   |<---prev_extent---><---newex--->|<---found_ext--->|....|<---ext2--->|
            //           0                  10          20                 30    40          50
            // merge:    |<---newex--->|<---found_ext--->|....|<---ext2--->|
            //           0            20                30    40          50
            if pos > 0 && (newex.first_block + newex.block_count as u32) < ex.first_block {
                if let Ok(mut prev_extent) = self.get_extent_from_node(node, pos - 1) {
                    if self.can_merge(&prev_extent, newex) {
                        self.merge_extent(&search_path, &mut prev_extent, newex)?;
                        return Ok(());
                    }
                }
            }
        }

        // Check if there's space to insert the new extent
        //                full         full
        // Before:   |<---ext1--->|<---ext2--->|
        //           10           20          30

        //                full          full
        // insert:   |<---ext1--->|<---ext2--->|<---newex--->|
        //           10           20           30           35
        if header.entries_count < header.max_entries_count {
            log::info!("[insert_extent] Node has space, inserting new extent");
            self.insert_new_extent(inode_ref, &mut search_path, newex)?;
        } else {
            log::info!("[insert_extent] Node is full (entries={}, max={}), creating new leaf", 
                header.entries_count, header.max_entries_count);
            self.create_new_leaf(inode_ref, &mut search_path, newex)?;
        }

        log::info!("[insert_extent] Completed - Final tree state: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);

        Ok(())
    }

    /// Get extent from the node at the given position.
    fn get_extent_from_node(&self, node: &ExtentPathNode, pos: usize) -> Result<Ext4Extent> {
        let data = self
            .block_device
            .read_offset(node.pblock as usize * BLOCK_SIZE);
        let extent_node = ExtentNode::load_from_data(&data, false).unwrap();

        match extent_node.get_extent(pos) {
            Some(extent) => Ok(extent),
            None => return_errno_with_message!(Errno::EINVAL, "Failed to get extent from node"),
        }
    }

    /// Get index from the node at the given position.
    fn get_index_from_node(&self, node: &ExtentPathNode, pos: usize) -> Result<Ext4ExtentIndex> {
        let data = self
            .block_device
            .read_offset(node.pblock as usize * BLOCK_SIZE);
        let extent_node = ExtentNode::load_from_data(&data, false).unwrap();

        extent_node.get_index(pos)
    }


    /// Check if two extents can be merged.
    ///
    /// This function determines whether two extents, `ex1` and `ex2`, can be merged
    /// into a single extent. Extents are contiguous ranges of blocks in the ext4
    /// filesystem that map logical block numbers to physical block numbers.
    ///
    /// # Arguments
    ///
    /// * `ex1` - The first extent to check.
    /// * `ex2` - The second extent to check.
    ///
    /// # Returns
    ///
    /// * `true` if the extents can be merged.
    /// * `false` otherwise.
    ///
    /// # Merge Conditions
    ///
    /// 1. **Same Unwritten State**:
    ///    - The `is_unwritten` state of both extents must be the same.
    ///    - Unwritten extents are placeholders for blocks that are allocated but not initialized.
    ///
    /// 2. **Contiguous Block Ranges**:
    ///    - The logical block range of the first extent must immediately precede
    ///      the logical block range of the second extent.
    ///
    /// 3. **Maximum Length**:
    ///    - The total length of the merged extent must not exceed the maximum allowed
    ///      extent length (`EXT_INIT_MAX_LEN`).
    ///    - If the extents are unwritten, the total length must also not exceed
    ///      the maximum length for unwritten extents (`EXT_UNWRITTEN_MAX_LEN`).
    ///
    /// 4. **Contiguous Physical Blocks**:
    ///    - The physical block range of the first extent must immediately precede
    ///      the physical block range of the second extent. This ensures that the
    ///      physical storage is contiguous.
    fn can_merge(&self, ex1: &Ext4Extent, ex2: &Ext4Extent) -> bool {
        // Check if the extents have the same unwritten state
        if ex1.is_unwritten() != ex2.is_unwritten() {
            return false;
        }
        let ext1_ee_len = ex1.get_actual_len() as usize;
        let ext2_ee_len = ex2.get_actual_len() as usize;
        
        // Check if the block ranges are contiguous
        if ex1.first_block + ext1_ee_len as u32 != ex2.first_block {
            return false;
        }

        // Check if the merged length would exceed the maximum allowed length
        if ext1_ee_len + ext2_ee_len > EXT_INIT_MAX_LEN as usize{
            return false;
        }

        // Check if the physical blocks are contiguous
        if ex1.get_pblock() + ext1_ee_len as u64 == ex2.get_pblock() {
            return true;
        }
        false
    }


    fn merge_extent(
        &self,
        search_path: &SearchPath,
        left_ext: &mut Ext4Extent,
        right_ext: &Ext4Extent,
    ) -> Result<()> {
        let depth = search_path.depth as usize;
        
        log::info!("[merge_extent] Merging extents at depth {}", depth);
        log::info!("[merge_extent] Left extent: logical block {}, physical block {}, length {}", 
            left_ext.first_block, left_ext.get_pblock(), left_ext.get_actual_len());
        log::info!("[merge_extent] Right extent: logical block {}, physical block {}, length {}", 
            right_ext.first_block, right_ext.get_pblock(), right_ext.get_actual_len());

        let unwritten = left_ext.is_unwritten();
        let len = left_ext.get_actual_len() + right_ext.get_actual_len();
        left_ext.set_actual_len(len);
        if unwritten {
            left_ext.mark_unwritten();
        }
        let header = search_path.path[depth].header;

        log::info!("[merge_extent] Merged extent: logical block {}, physical block {}, new length {}", 
            left_ext.first_block, left_ext.get_pblock(), left_ext.get_actual_len());

        if header.max_entries_count > 4 {
            let node = &search_path.path[depth];
            let block = node.pblock_of_node;
            let new_ex_offset = core::mem::size_of::<Ext4ExtentHeader>() + core::mem::size_of::<Ext4Extent>() * (node.position);
            let mut ext4block = Block::load(&self.block_device, block * BLOCK_SIZE);
            let left_ext:&mut Ext4Extent = ext4block.read_offset_as_mut(new_ex_offset);

            let unwritten = left_ext.is_unwritten();
            let len = left_ext.get_actual_len() + right_ext.get_actual_len();
            left_ext.set_actual_len(len);
            if unwritten {
                left_ext.mark_unwritten();
            }

            log::info!("[merge_extent] Updated on-disk extent: logical block {}, physical block {}, length {}", 
                left_ext.first_block, left_ext.get_pblock(), left_ext.get_actual_len());

            ext4block.sync_blk_to_disk(&self.block_device);
            log::info!("[merge_extent] Synced merged extent to disk");
        }

        Ok(())
    }

    fn insert_new_extent(
        &self,
        inode_ref: &mut Ext4InodeRef,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        let depth = search_path.depth as usize;
        let node = &mut search_path.path[depth]; // Get the node at the current depth
        let header = node.header;

        log::info!("[insert_new_extent] Inserting extent at depth {}: logical block {}, physical block {}, length {}", 
            depth, new_extent.first_block, new_extent.get_pblock(), new_extent.get_actual_len());
        log::info!("[insert_new_extent] Node info: entries={}, max={}, position={}", 
            header.entries_count, header.max_entries_count, node.position);
        
        log::debug!("[insert_new_extent] New extent details:");
        log::debug!("  - Logical start block: {}", new_extent.first_block);
        log::debug!("  - Physical start block: {}", new_extent.get_pblock());
        log::debug!("  - Block count: {}", new_extent.block_count);
        log::debug!("  - Actual length: {}", new_extent.get_actual_len());
        log::debug!("  - Unwritten: {}", new_extent.is_unwritten());
        log::debug!("  - Raw data: start_lo={}, start_hi={}, block_count={:#x}", 
            new_extent.start_lo, new_extent.start_hi, new_extent.block_count);
        log::debug!("  - Tree position: depth={}, position={}, at_root={}", 
            depth, node.position, node.pblock_of_node == 0);

        // insert at root
        if depth == 0 {
            // Node is empty (no extents)
            if header.entries_count == 0 {
                log::info!("[insert_new_extent] Inserting first extent into empty root node");
                *inode_ref.inode.root_extent_mut_at(node.position) = *new_extent;
                inode_ref.inode.root_extent_header_mut().entries_count += 1;

                self.write_back_inode(inode_ref);
                
                // Add debug logs after successful insertion at root node
                log::debug!("[insert_new_extent] Successfully inserted at root:");
                log::debug!("  - Root header: magic={:x}, entries={}, max={}, depth={}", 
                    inode_ref.inode.root_extent_header().magic,
                    inode_ref.inode.root_extent_header().entries_count,
                    inode_ref.inode.root_extent_header().max_entries_count,
                    inode_ref.inode.root_extent_header().depth);
                
                return Ok(());
            }
            // Check if root node is full, need to grow in depth
            if header.entries_count == header.max_entries_count {
                log::info!("[insert_new_extent] Root node full, growing in depth");
                self.ext_grow_indepth(inode_ref)?;
                // After growing, re-insert
                return self.insert_extent(inode_ref, new_extent);
            }

            
            // Not empty, insert at search result pos + 1
            log::info!("[insert_new_extent] Inserting at root at position {} (entries: {})", 
                node.position + 1, header.entries_count);
            *inode_ref.inode.root_extent_mut_at(node.position + 1) = *new_extent;
            inode_ref.inode.root_extent_header_mut().entries_count += 1;
            
            log::debug!("[insert_new_extent] Successfully inserted at root:");
            log::debug!("  - Root header: magic={:x}, entries={}, max={}, depth={}", 
                inode_ref.inode.root_extent_header().magic,
                inode_ref.inode.root_extent_header().entries_count,
                inode_ref.inode.root_extent_header().max_entries_count,
                inode_ref.inode.root_extent_header().depth);
            
            return Ok(());
        } else {
            // insert at nonroot
            log::info!("[insert_new_extent] Inserting at non-root node at depth {}, position {}", 
                depth, node.position + 1);

            // load block
            let node_block = node.pblock_of_node;
            let mut ext4block =
            Block::load(&self.block_device, node_block * BLOCK_SIZE);
            let new_ex_offset = core::mem::size_of::<Ext4ExtentHeader>() + core::mem::size_of::<Ext4Extent>() * (node.position + 1);

            // insert new extent
            let ex: &mut Ext4Extent = ext4block.read_offset_as_mut(new_ex_offset);
            *ex = *new_extent;
            let header: &mut Ext4ExtentHeader = ext4block.read_offset_as_mut(0);

            // update entry count 
            header.entries_count += 1;
            log::info!("[insert_new_extent] Updated non-root node: entries={}, max={}", 
                header.entries_count, header.max_entries_count);

            // Complete block processing and sync to disk first
            let node_header_entries = header.entries_count;
            let node_header_max = header.max_entries_count;
            ext4block.sync_blk_to_disk(&self.block_device);
            
            // Set the checksum for the updated extent block
            if let Err(e) = self.set_extent_block_checksum(inode_ref, node_block) {
                log::warn!("[insert_new_extent] Failed to set extent block checksum: {:?}", e);
            } else {
                log::info!("[insert_new_extent] Set checksum for updated extent block");
            }
            
            log::info!("[insert_new_extent] Synced non-root node to disk");

            log::debug!("[insert_new_extent] Successfully inserted at non-root node:");
            log::debug!("  - Node header: entries={}, max={}, depth={}", 
                node_header_entries, node_header_max, depth);
            log::debug!("  - Block address: {}", node_block);
            log::debug!("  - Extent position: {}", node.position + 1);
            log::debug!("  - Extent: logical={}, physical={}, length={}", 
                new_extent.first_block, new_extent.get_pblock(), new_extent.get_actual_len());

            return Ok(());
        }

        return_errno_with_message!(Errno::ENOTSUP, "Not supported insert extent at nonroot");
    }

    // finds empty index and adds new leaf. if no free index is found, then it requests in-depth growing.
    fn create_new_leaf(
        &self,
        inode_ref: &mut Ext4InodeRef,
        search_path: &mut SearchPath,
        new_extent: &mut Ext4Extent,
    ) -> Result<()> {
        log::info!("[create_new_leaf] Starting - Current tree state:");
        log::info!("[create_new_leaf] Root header: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);
        log::info!("[create_new_leaf] New extent: logical block {}, physical block {}, length {}", 
            new_extent.first_block, new_extent.get_pblock(), new_extent.get_actual_len());
        
        // tree is full, time to grow in depth
        log::info!("[create_new_leaf] Tree is full, calling ext_grow_indepth");
        self.ext_grow_indepth(inode_ref)?;
        
        log::info!("[create_new_leaf] After ext_grow_indepth - New tree state:");
        log::info!("[create_new_leaf] Root header: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);

        // insert again
        log::info!("[create_new_leaf] Attempting to insert extent again");
        self.insert_extent(inode_ref, new_extent)
    }

    
    // allocates new block
    // moves top-level data (index block or leaf) into the new block
    // initializes new top-level, creating index that points to the
    // just created block
    fn ext_grow_indepth(&self, inode_ref: &mut Ext4InodeRef) -> Result<()>{
        log::info!("[ext_grow_indepth] Starting - Current tree state:");
        log::info!("[ext_grow_indepth] Root header: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);

        // Allocate new block to store original root node content
        let new_block = self.balloc_alloc_block(inode_ref, None)?;
        log::info!("[ext_grow_indepth] Allocated new block: {}", new_block);

        // Load new block
        let mut new_ext4block =
            Block::load(&self.block_device, new_block as usize * BLOCK_SIZE);
        log::info!("[ext_grow_indepth] Loaded new block");

        // Clear new block to ensure no garbage data
        new_ext4block.data.fill(0);

        // Save original root node information
        let old_root_header = inode_ref.inode.root_extent_header();
        let old_depth = old_root_header.depth;
        let old_entries_count = old_root_header.entries_count;
        
        // Get logical block number of first extent (only when original was a leaf node)
        let first_logical_block = if old_depth == 0 && old_entries_count > 0 {
            inode_ref.inode.root_extent_at(0).first_block
        } else {
            0
        };

        // Copy root node extents data to new block
        // extent start position in inode block is 12 bytes (after header)
        // extent start position in new block is also 12 bytes (after header)
        let header_size = EXT4_EXTENT_HEADER_SIZE;
        
        // Copy header first
        let mut new_header = Ext4ExtentHeader::new(
            EXT4_EXTENT_MAGIC,
            old_entries_count,
            ((BLOCK_SIZE - header_size) / EXT4_EXTENT_SIZE) as u16, // Maximum entries the new block can hold
            0, // New block becomes a leaf node, depth 0
            0  // generation field, usually 0
        );
        
        // Write header to new block
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &new_header as *const _ as *const u8,
                header_size
            )
        };
        new_ext4block.data[..header_size].copy_from_slice(header_bytes);
        
        // Copy extents data
        if old_entries_count > 0 {
            // Copy extents from root block to new block
            // extent start position in inode block is 12 bytes (after header)
            // extent start position in new block is also 12 bytes (after header)
            let root_extents_size = old_entries_count as usize * EXT4_EXTENT_SIZE;
            
            // Use temporary variable to store block data to avoid mutable borrow conflicts
            let block_data = unsafe {
                let block_ptr = inode_ref.inode.block.as_ptr();
                core::slice::from_raw_parts(block_ptr as *const u8, 60)
            };
            
            let root_extents_bytes = &block_data[header_size..header_size + root_extents_size];
            new_ext4block.data[header_size..header_size + root_extents_size]
                .copy_from_slice(root_extents_bytes);
        }
        
        log::info!("[ext_grow_indepth] Copied root data to new block and set header: magic={:x}, entries={}, max_entries={}, depth={}",
            new_header.magic, new_header.entries_count, new_header.max_entries_count, new_header.depth);
        
        // Set checksum for the new extent block
        new_ext4block.sync_blk_to_disk(&self.block_device);
        // Set the checksum for the new extent block
        if let Err(e) = self.set_extent_block_checksum(inode_ref, new_block as usize) {
            log::warn!("[ext_grow_indepth] Failed to set extent block checksum: {:?}", e);
        } else {
            log::info!("[ext_grow_indepth] Set checksum for new extent block");
        }
        
        // First read the block number of the first extent (if any), then update root node
        let first_logical_block_saved = first_logical_block;
        
        // Update root node to be an index node
        {
            let mut root_header = inode_ref.inode.root_extent_header_mut();
            root_header.set_magic(); // Set magic
            root_header.set_entries_count(1); // Index node initially has one entry
            root_header.set_max_entries_count(4); // Root index node typically has 4 entries
            root_header.add_depth(); // Increase depth
            
            log::info!("[ext_grow_indepth] Updated root header: depth {} -> {}, entries={}, max={}", 
                old_depth, root_header.depth, root_header.entries_count, root_header.max_entries_count);
        }
        
        // Clear extents data in original root node
        unsafe {
            let root_block_ptr = inode_ref.inode.block.as_mut_ptr() as *mut u8;
            // Skip header part, only clear the extent data after it
            let extents_ptr = root_block_ptr.add(header_size);
            core::ptr::write_bytes(extents_ptr, 0, 60 - header_size);
        }
        
        // Create first index entry in root node pointing to new block
        {
            let mut root_first_index = inode_ref.inode.root_first_index_mut();
            root_first_index.first_block = first_logical_block_saved; // Set starting logical block number
            root_first_index.store_pblock(new_block); // Store physical address of new block
            
            log::info!("[ext_grow_indepth] Root became index block, first_block={}, pointing to block {}", 
                first_logical_block_saved, new_block);
        }

        // Write updated inode back to disk
        self.write_back_inode(inode_ref);
        log::info!("[ext_grow_indepth] Wrote updated inode back to disk");

        log::info!("[ext_grow_indepth] Completed - Final tree state:");
        log::info!("[ext_grow_indepth] Root header: magic={:x}, entries={}, max={}, depth={}", 
            inode_ref.inode.root_extent_header().magic,
            inode_ref.inode.root_extent_header().entries_count,
            inode_ref.inode.root_extent_header().max_entries_count,
            inode_ref.inode.root_extent_header().depth);

        Ok(())
    }

}

impl Ext4 {
    // Assuming init state
    // depth 0 (root node)
    // +--------+--------+--------+
    // |  idx1  |  idx2  |  idx3  |
    // +--------+--------+--------+
    //     |         |         |
    //     v         v         v
    //
    // depth 1 (internal node)
    // +--------+...+--------+  +--------+...+--------+ ......
    // |  idx1  |...|  idxn  |  |  idx1  |...|  idxn  | ......
    // +--------+...+--------+  +--------+...+--------+ ......
    //     |           |         |             |
    //     v           v         v             v
    //
    // depth 2 (leaf nodes)
    // +--------+...+--------+  +--------+...+--------+  ......
    // | ext1   |...| extn   |  | ext1   |...| extn   |  ......
    // +--------+...+--------+  +--------+...+--------+  ......
    pub fn extent_remove_space(
        &self,
        inode_ref: &mut Ext4InodeRef,
        from: u32,
        to: u32,
    ) -> Result<usize> {
        // log::info!("Remove space from {:x?} to {:x?}", from, to);
        let mut search_path = self.find_extent(inode_ref, from)?;

        // for i in search_path.path.iter() {
        //     log::info!("from Path: {:x?}", i);
        // }

        let depth = search_path.depth as usize;

        /* If we do remove_space inside the range of an extent */
        let mut ex = search_path.path[depth].extent.unwrap();
        if ex.get_first_block() < from
            && to < (ex.get_first_block() + ex.get_actual_len() as u32 - 1)
        {
            let mut newex = Ext4Extent::default();
            let unwritten = ex.is_unwritten();
            let ee_block = ex.first_block;
            let block_count = ex.block_count;
            let newblock = to + 1 - ee_block + ex.get_pblock() as u32;
            ex.block_count = from as u16 - ee_block as u16;

            if unwritten {
                ex.mark_unwritten();
            }
            newex.first_block = to + 1;
            newex.block_count = (ee_block + block_count as u32 - 1 - to) as u16;
            newex.start_lo = newblock;
            newex.start_hi = ((newblock as u64) >> 32) as u16;

            self.insert_extent(inode_ref, &mut newex)?;

            return Ok(EOK);
        }

        // log::warn!("Remove space in depth: {:x?}", depth);

        let mut i = depth as isize;

        while i >= 0 {
            // we are at the leaf node
            // depth 0 (root node)
            // +--------+--------+--------+
            // |  idx1  |  idx2  |  idx3  |
            // +--------+--------+--------+
            //              |path
            //              v
            //              idx2
            // depth 1 (internal node)
            // +--------+--------+--------+ ......
            // |  idx1  |  idx2  |  idx3  | ......
            // +--------+--------+--------+ ......
            //              |path
            //              v
            //              ext2
            // depth 2 (leaf nodes)
            // +--------+--------+..+--------+
            // | ext1   | ext2   |..|last_ext|
            // +--------+--------+..+--------+
            //            ^            ^
            //            |            |
            //            from         to(exceed last ext, rest of the extents will be removed)
            if i as usize == depth {
                let node_pblock = search_path.path[i as usize].pblock_of_node;

                let header = search_path.path[i as usize].header;
                let entries_count = header.entries_count;

                // we are at root
                if node_pblock == 0 {
                    let first_ex = inode_ref.inode.root_extent_at(0);
                    let last_ex = inode_ref.inode.root_extent_at(entries_count as usize - 1);

                    let mut leaf_from = first_ex.first_block;
                    let mut leaf_to = last_ex.first_block + last_ex.get_actual_len() as u32 - 1;
                    if leaf_from < from {
                        leaf_from = from;
                    }
                    if leaf_to > to {
                        leaf_to = to;
                    }
                    // log::trace!("from {:x?} to {:x?} leaf_from {:x?} leaf_to {:x?}", from, to, leaf_from, leaf_to);
                    self.ext_remove_leaf(inode_ref, &mut search_path, leaf_from, leaf_to)?;

                    i -= 1;
                    continue;
                }
                let ext4block =
                    Block::load(&self.block_device, node_pblock * BLOCK_SIZE);

                let header = search_path.path[i as usize].header;
                let entries_count = header.entries_count;

                let first_ex: Ext4Extent = ext4block.read_offset_as(size_of::<Ext4ExtentHeader>());
                let last_ex: Ext4Extent = ext4block.read_offset_as(
                    size_of::<Ext4ExtentHeader>()
                        + size_of::<Ext4Extent>() * (entries_count - 1) as usize,
                );

                let mut leaf_from = first_ex.first_block;
                let mut leaf_to = last_ex.first_block + last_ex.get_actual_len() as u32 - 1;

                if leaf_from < from {
                    leaf_from = from;
                }
                if leaf_to > to {
                    leaf_to = to;
                }
                // log::trace!(
                //     "from {:x?} to {:x?} leaf_from {:x?} leaf_to {:x?}",
                //     from,
                //     to,
                //     leaf_from,
                //     leaf_to
                // );

                self.ext_remove_leaf(inode_ref, &mut search_path, leaf_from, leaf_to)?;

                i -= 1;
                continue;
            }

            // log::trace!("---at level---{:?}\n", i);

            // we are at index
            // example i=1, depth=2
            // depth 0 (root node) - Index node being processed
            // +--------+--------+--------+
            // |  idx1  |  idx2  |  idx3  |
            // +--------+--------+--------+
            //            |path     | Next node to process (more_to_rm?)
            //            v         v
            //           idx2
            //
            // depth 1 (internal node)
            // +--------++--------+...+--------+
            // |  idx1  ||  idx2  |...|  idxn  |
            // +--------++--------+...+--------+
            //            |path
            //            v
            //            ext2
            // depth 2 (leaf nodes)
            // +--------+--------+..+--------+
            // | ext1   | ext2   |..|last_ext|
            // +--------+--------+..+--------+
            let header = search_path.path[i as usize].header;
            if self.more_to_rm(&search_path.path[i as usize], to) {
                // todo
                // load next idx

                // go to this node's child
                i += 1;
            } else {
                if i > 0 {
                    // empty
                    if header.entries_count == 0 {
                        self.ext_remove_idx(inode_ref, &mut search_path, i as u16 - 1)?;
                    }
                }

                let idx = i;
                if idx - 1 < 0 {
                    break;
                }
                i -= 1;
            }
        }

        Ok(EOK)
    }

    pub fn ext_remove_leaf(
        &self,
        inode_ref: &mut Ext4InodeRef,
        path: &mut SearchPath,
        from: u32,
        to: u32,
    ) -> Result<usize> {
        // log::trace!("Remove leaf from {:x?} to {:x?}", from, to);

        // depth 0 (root node)
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //     |         |         |
        //     v         v         v
        //     ^
        //     Current position
        let depth = inode_ref.inode.root_header_depth();
        let mut header = path.path[depth as usize].header;

        let mut new_entry_count = header.entries_count;
        let mut ex2 = Ext4Extent::default();

        /* find where to start removing */
        let pos = path.path[depth as usize].position;
        let entry_count = header.entries_count;

        // depth 1 (internal node)
        // +--------+...+--------+  +--------+...+--------+ ......
        // |  idx1  |...|  idxn  |  |  idx1  |...|  idxn  | ......
        // +--------+...+--------+  +--------+...+--------+ ......
        //     |           |         |             |
        //     v           v         v             v
        //     ^
        //     Current loaded node

        // load node data
        let node_disk_pos = path.path[depth as usize].pblock_of_node * BLOCK_SIZE;

        let mut ext4block = if node_disk_pos == 0 {
            // we are at root
            Block::load_inode_root_block(&inode_ref.inode.block)
        } else {
            Block::load(&self.block_device, node_disk_pos)
        };

        // depth 2 (leaf nodes)
        // +--------+...+--------+  +--------+...+--------+  ......
        // | ext1   |...| extn   |  | ext1   |...| extn   |  ......
        // +--------+...+--------+  +--------+...+--------+  ......
        //     ^
        //     Current start extent

        // start from pos
        for i in pos..entry_count as usize {
            let ex: &mut Ext4Extent = ext4block
                .read_offset_as_mut(size_of::<Ext4ExtentHeader>() + i * size_of::<Ext4Extent>());

            if ex.first_block > to {
                break;
            }

            let mut new_len = 0;
            let mut start = ex.first_block;
            let mut new_start = ex.first_block;

            let mut len = ex.get_actual_len();
            let mut newblock = ex.get_pblock();

            if start + len as u32 - 1 < from {
                continue;
            }

            // Initial state:
            // +--------+...+--------+  +--------+...+--------+  ......
            // | ext1   |...| ext2   |  | ext3   |...| extn   |  ......
            // +--------+...+--------+  +--------+...+--------+  ......
            //               ^                    ^
            //              from                  to

            // Case 1: Remove a portion within the extent
            if start < from {
                len -= from as u16 - start as u16;
                new_len = from - start;
                start = from;
            } else {
                // Case 2: Adjust extent that partially overlaps the 'to' boundary
                if start + len as u32 - 1 > to {
                    new_len = start + len as u32 - 1 - to;
                    len -= new_len as u16;
                    new_start = to + 1;
                    newblock += (to + 1 - start) as u64;
                    ex2 = *ex;
                }
            }

            // After removing range from `from` to `to`:
            // +--------+...+--------+  +--------+...+--------+  ......
            // | ext1   |...[removed]|  |[removed]|...| extn   |  ......
            // +--------+...+--------+  +--------+...+--------+  ......
            //               ^                    ^
            //              from                  to
            //                                  new_start

            // Remove blocks within the extent
            self.ext_remove_blocks(inode_ref, ex, start, start + len as u32 - 1);

            ex.first_block = new_start;
            // log::trace!("after remove leaf ex first_block {:x?}", ex.first_block);

            if new_len == 0 {
                new_entry_count -= 1;
            } else {
                let unwritten = ex.is_unwritten();
                ex.store_pblock(newblock as u64);
                ex.block_count = new_len as u16;

                if unwritten {
                    ex.mark_unwritten();
                }
            }
        }

        // Move remaining extents to the start:
        // Before:
        // +--------+--------+...+--------+
        // | ext3   | ext4   |...| extn   |
        // +--------+--------+...+--------+
        //      ^       ^
        //      rm      rm
        // After:
        // +--------+.+--------+--------+...
        // | ext1   |.| extn   | [empty]|...
        // +--------+.+--------+--------+...

        // Move any remaining extents to the starting position of the node.
        if ex2.first_block > 0 {
            let start_index = size_of::<Ext4ExtentHeader>() + pos * size_of::<Ext4Extent>();
            let end_index =
                size_of::<Ext4ExtentHeader>() + entry_count as usize * size_of::<Ext4Extent>();
            let remaining_extents: Vec<u8> = ext4block.data[start_index..end_index].to_vec();
            ext4block.data[size_of::<Ext4ExtentHeader>()
                ..size_of::<Ext4ExtentHeader>() + remaining_extents.len()]
                .copy_from_slice(&remaining_extents);
        }

        // Update the entries count in the header
        header.entries_count = new_entry_count;

        let block_header: &mut Ext4ExtentHeader = ext4block.read_offset_as_mut(0);
        block_header.entries_count = new_entry_count;

        if node_disk_pos == 0 {
            let data = unsafe {
                let ptr = ext4block.data.as_ptr() as *const u32;
                core::slice::from_raw_parts(ptr, 15)
            };
            inode_ref.inode.block.copy_from_slice(data);
            self.write_back_inode(inode_ref);
        } else {
            ext4block.sync_blk_to_disk(&self.block_device);
            if let Err(e) = self.set_extent_block_checksum(inode_ref, path.path[depth as usize].pblock_of_node) {
                log::warn!("Failed to set extent block checksum: {:?}", e);
            }
        }

        /*
         * If the extent pointer is pointed to the first extent of the node, and
         * there's still extents presenting, we may need to correct the indexes
         * of the paths.
         */
        if pos == 0 && new_entry_count > 0 {
            self.ext_correct_indexes(path)?;
        }

        /* if this leaf is free, then we should
         * remove it from index block above */
        if new_entry_count == 0 {
            // if we are at root?
            if path.path[depth as usize].pblock_of_node == 0 {
                return Ok(EOK);
            }
            self.ext_remove_idx(inode_ref, path, depth - 1)?;
        } else if depth > 0 {
            // go to next index
            path.path[depth as usize - 1].position += 1;
        }

        Ok(EOK)
    }

    fn ext_remove_index_block(&self, inode_ref: &mut Ext4InodeRef, index: &mut Ext4ExtentIndex) {
        let block_to_free = index.get_pblock();

        // log::trace!("remove index's block {:x?}", block_to_free);
        self.balloc_free_blocks(inode_ref, block_to_free as _, 1);
    }

    fn ext_remove_idx(
        &self,
        inode_ref: &mut Ext4InodeRef,
        path: &mut SearchPath,
        depth: u16,
    ) -> Result<usize> {
        // log::trace!("Remove index at depth {:x?}", depth);

        // Initial state:
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //           ^
        // Current index to remove (pos=1)

        // Removing index:
        // +--------+--------+--------+
        // |  idx1  |[empty] |  idx3  |
        // +--------+--------+--------+
        //           ^

        let i = depth as usize;
        let mut header = path.path[i].header;

        // Get the index block to delete
        let leaf_block = path.path[i].index.unwrap().get_pblock();

        let node_pblock = path.path[i].pblock_of_node;
        let node_disk_pos = node_pblock * BLOCK_SIZE;
        let mut ext4block = if node_disk_pos == 0 {
            Block::load_inode_root_block(&inode_ref.inode.block)
        } else {
            Block::load(&self.block_device, node_disk_pos)
        };

        // If current index is not the last one, move subsequent indexes forward
        if path.path[i].position != header.entries_count as usize - 1 {
            let start_pos = size_of::<Ext4ExtentHeader>()
                + path.path[i].position * size_of::<Ext4ExtentIndex>();
            let end_pos = size_of::<Ext4ExtentHeader>()
                + (header.entries_count as usize) * size_of::<Ext4ExtentIndex>();

            let remaining_indexes: Vec<u8> =
                ext4block.data[start_pos + size_of::<Ext4ExtentIndex>()..end_pos].to_vec();
            ext4block.data[start_pos..start_pos + remaining_indexes.len()]
                .copy_from_slice(&remaining_indexes);
            let remaining_size = remaining_indexes.len();

            // Clear the remaining positions
            let empty_start = start_pos + remaining_size;
            let empty_end = end_pos;
            ext4block.data[empty_start..empty_end].fill(0);
        }

        // Update the entries_count in the header
        header.entries_count -= 1;
        let block_header: &mut Ext4ExtentHeader = ext4block.read_offset_as_mut(0);
        block_header.entries_count = header.entries_count;

        if node_disk_pos == 0 {
            let data = unsafe {
                let ptr = ext4block.data.as_ptr() as *const u32;
                core::slice::from_raw_parts(ptr, 15)
            };
            inode_ref.inode.block.copy_from_slice(data);
            self.write_back_inode(inode_ref);
        } else {
            ext4block.sync_blk_to_disk(&self.block_device);
            if let Err(e) = self.set_extent_block_checksum(inode_ref, node_pblock) {
                log::warn!("Failed to set extent block checksum: {:?}", e);
            }
        }

        // Free the index block
        self.ext_remove_index_block(inode_ref, &mut path.path[i].index.unwrap());

        // If we're not at the root, check if we need to update the parent node index
        let mut idx = i;
        while idx > 0 {
            if path.path[idx].position != 0 {
                break;
            }

            let parent_idx = idx - 1;
            let parent_index = &mut path.path[parent_idx].index.unwrap();
            let current_index = &path.path[idx].index.unwrap();

            parent_index.first_block = current_index.first_block;
            self.write_back_inode(inode_ref);

            idx -= 1;
        }

        Ok(EOK)
    }

    /// Correct the first block of the parent index.
    fn ext_correct_indexes(&self, path: &mut SearchPath) -> Result<usize> {
        // If child gets removed from parent, we need to update the parent's first_block
        let mut depth = path.depth as usize;
        
        // depth 2:
        // +--------+--------+--------+
        // |[empty] |  ext2  |  ext3  |
        // +--------+--------+--------+
        // ^
        // pos=0, ext1_first_block=0(removed) parent index first block=0

        // depth 2:
        // +--------+--------+--------+
        // |  ext2  |  ext3  |[empty] |
        // +--------+--------+--------+
        // ^
        // pos=0, now first_block=ext2_first_block

        // Update parent node index:
        // depth 1:
        // +-----------------------+
        // | idx1_2 |...| idx1_n   |
        // +-----------------------+
        //     ^
        //     Update parent node index (first_block)

        // depth 0:
        // +--------+--------+--------+
        // |  idx1  |  idx2  |  idx3  |
        // +--------+--------+--------+
        //     |
        //     Update root node index (first_block)

        while depth > 0 {
            let parent_idx = depth - 1;
            
            // Get the extent at the current level
            if let Some(child_extent) = path.path[depth].extent {
                // Get the parent node
                let parent_node = &mut path.path[parent_idx];
                // Get parent node's index and update first_block
                if let Some(ref mut parent_index) = parent_node.index {
                    parent_index.first_block = child_extent.first_block;
                }
            }

            depth -= 1;
        }

        Ok(EOK)
    }

    fn ext_remove_blocks(
        &self,
        inode_ref: &mut Ext4InodeRef,
        ex: &mut Ext4Extent,
        from: u32,
        to: u32,
    ) {
        let len = to - from + 1;
        let num = from - ex.first_block;
        let start: u32 = ex.get_pblock() as u32 + num;
        self.balloc_free_blocks(inode_ref, start as _, len);
    }

    pub fn more_to_rm(&self, path: &ExtentPathNode, to: u32) -> bool {
        let header = path.header;

        // No Sibling exists
        if header.entries_count == 1 {
            return false;
        }

        let pos = path.position;
        if pos > header.entries_count as usize - 1 {
            return false;
        }

        // Check if index is out of bounds
        if let Some(index) = path.index {
            let last_index_pos = header.entries_count as usize - 1;
            let node_disk_pos = path.pblock_of_node * BLOCK_SIZE;
            let ext4block = Block::load(&self.block_device, node_disk_pos);
            let last_index: Ext4ExtentIndex =
                ext4block.read_offset_as(size_of::<Ext4ExtentIndex>() * last_index_pos);

            if path.position > last_index_pos || index.first_block > last_index.first_block {
                return false;
            }

            // Check if index's first_block is greater than 'to'
            if index.first_block > to {
                return false;
            }
        }

        true
    }
}

impl Ext4 {
    /// Calculate and set the extent block checksum in the extent tail
    fn set_extent_block_checksum(&self, inode_ref: &Ext4InodeRef, block_addr: usize) -> Result<()> {
        // Check if metadata checksums are enabled in the filesystem
        let features_ro_compat = self.super_block.features_read_only;
        // EXT4_FEATURE_RO_COMPAT_METADATA_CSUM is typically 0x400
        let has_metadata_checksums = (features_ro_compat & 0x400) != 0;
        
        if !has_metadata_checksums {
            return Ok(());
        }

        // Load the extent block
        let mut ext4block = Block::load(&self.block_device, block_addr * BLOCK_SIZE);
        
        // Get the extent header
        let header = ext4block.read_offset_as::<Ext4ExtentHeader>(0);
        
        // Check for valid magic
        if header.magic != EXT4_EXTENT_MAGIC {
            return_errno_with_message!(Errno::EINVAL, "Invalid extent magic");
        }
        
        // Calculate position of the extent tail
        let tail_offset = ext4_extent_tail_offset(&header);
        
        // Create a copy of the data for checksum calculation to avoid borrow conflicts
        let data_for_checksum = ext4block.data[..tail_offset].to_vec();
        
        // Calculate checksum
        let checksum = self.calculate_extent_block_checksum(inode_ref, &data_for_checksum, block_addr);
        
        // Get a mutable reference to the tail
        let tail: &mut Ext4ExtentTail = ext4block.read_offset_as_mut(tail_offset);
        
        // Set checksum in tail
        tail.et_checksum = checksum;
        
        // Write back the block
        ext4block.sync_blk_to_disk(&self.block_device);
        
        Ok(())
    }
    
    /// Calculate the checksum for an extent block
    fn calculate_extent_block_checksum(&self, inode_ref: &Ext4InodeRef, data: &[u8], block_addr: usize) -> u32 {
        let mut checksum = 0;
        
        // If metadata checksums are not enabled, return 0
        let features_ro_compat = self.super_block.features_read_only;
        // EXT4_FEATURE_RO_COMPAT_METADATA_CSUM is typically 0x400
        let has_metadata_checksums = (features_ro_compat & 0x400) != 0;
        
        if !has_metadata_checksums {
            return 0;
        }
        
        // Get UUID from superblock
        let uuid = &self.super_block.uuid;
        
        // Calculate checksum - first using UUID
        checksum = ext4_crc32c(EXT4_CRC32_INIT, uuid, uuid.len() as u32);
        
        // Add inode number to checksum
        let ino_index = inode_ref.inode_num;
        checksum = ext4_crc32c(checksum, &ino_index.to_le_bytes(), 4);
        
        // Add inode generation to checksum
        let ino_gen = inode_ref.inode.generation;
        checksum = ext4_crc32c(checksum, &ino_gen.to_le_bytes(), 4);
        
        // Finally add the extent block data
        checksum = ext4_crc32c(checksum, data, data.len() as u32);
        
        checksum
    }

}

/// Calculate the offset of the extent tail
pub fn ext4_extent_tail_offset(header: &Ext4ExtentHeader) -> usize {
    size_of::<Ext4ExtentHeader>() + 
    (header.max_entries_count as usize * size_of::<Ext4Extent>())
}