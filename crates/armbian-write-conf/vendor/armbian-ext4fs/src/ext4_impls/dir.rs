use crate::prelude::*;
use crate::return_errno_with_message;

use crate::ext4_defs::*;

impl Ext4 {
    /// Find a directory entry in a directory
    ///
    /// Params:
    /// parent_inode: u32 - inode number of the parent directory
    /// name: &str - name of the entry to find
    /// result: &mut Ext4DirSearchResult - result of the search
    ///
    /// Returns:
    /// `Result<usize>` - status of the search
    pub fn dir_find_entry(
        &self,
        parent_inode: u32,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> Result<usize> {
        // load parent inode
        let parent = self.get_inode_ref(parent_inode);
        assert!(parent.inode.is_dir());

        // start from the first logical block
        let mut iblock = 0;
        // physical block id
        let mut fblock: Ext4Fsblk = 0;

        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let total_blocks: u64 = inode_size / BLOCK_SIZE as u64;

        // iterate all blocks
        while iblock < total_blocks {
            let search_path = self.find_extent(&parent, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // get physical block id
                fblock = path.pblock;

                // load physical block
                let mut ext4block =
                    Block::load(&self.block_device, fblock as usize * BLOCK_SIZE);

                // find entry in block
                match self.dir_find_in_block(&ext4block, name, result) {
                    Ok(_) => {
                        result.pblock_id = fblock as usize;
                        return Ok(EOK);
                    }
                    Err(e) if e.error() != Errno::ENOENT => return Err(e),
                    Err(_) => {}
                }
            } else {
                return_errno_with_message!(Errno::ENOENT, "dir search fail")
            }
            // go to next block
            iblock += 1
        }

        return_errno_with_message!(Errno::ENOENT, "dir search fail");
    }

    /// Find a directory entry in a block
    ///
    /// Params:
    /// block: &mut Block - block to search in
    /// name: &str - name of the entry to find
    ///
    /// Returns:
    /// result: Ext4DirEntry - result of the search
    pub fn dir_find_in_block(
        &self,
        block: &Block,
        name: &str,
        result: &mut Ext4DirSearchResult,
    ) -> Result<Ext4DirEntry> {
        let mut offset = 0;
        let mut prev_de_offset = 0;

        // start from the first entry
        while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
            let (inode, rec_len, name_len) = dir_entry_header(&block.data, offset)?;
            let name_start = offset + size_of::<Ext4FakeDirEntry>();
            if inode != 0 && block.data[name_start..name_start + name_len] == *name.as_bytes() {
                let mut raw = [0u8; size_of::<Ext4DirEntry>()];
                let n = min(raw.len(), BLOCK_SIZE - offset);
                raw[..n].copy_from_slice(&block.data[offset..offset + n]);
                let de: Ext4DirEntry = unsafe { core::ptr::read_unaligned(raw.as_ptr() as *const _) };
                result.dentry = de;
                result.offset = offset;
                result.prev_offset = prev_de_offset;
                return Ok(de);
            }

            prev_de_offset = offset;
            offset += rec_len;
        }
        return_errno_with_message!(Errno::ENOENT, "dir find in block failed");
    }

    /// Get dir entries of a inode
    ///
    /// Params:
    /// inode: u32 - inode number of the directory
    ///
    /// Returns:
    /// `Vec<Ext4DirEntry>` - list of directory entries
    pub fn dir_get_entries(&self, inode: u32) -> Vec<Ext4DirEntry> {
        let mut entries = Vec::new();

        // load inode
        let inode_ref = self.get_inode_ref(inode);
        assert!(inode_ref.inode.is_dir());

        // calculate total blocks
        let inode_size = inode_ref.inode.size();
        let total_blocks = inode_size / BLOCK_SIZE as u64;

        // start from the first logical block
        let mut iblock = 0;

        // iterate all blocks
        while iblock < total_blocks {
            // get physical block id of a logical block id
            let search_path = self.find_extent(&inode_ref, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // get physical block id
                let fblock = path.pblock;

                // load physical block
                let ext4block =
                    Block::load(&self.block_device, fblock as usize * BLOCK_SIZE);
                let mut offset = 0;

                // iterate all entries in a block
                while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
                    let de: Ext4DirEntry = ext4block.read_offset_as(offset);
                    if !de.unused() {
                        entries.push(de);
                    }
                    offset += de.entry_len() as usize;
                }
            }

            // go ot next block
            iblock += 1;
        }
        entries
    }

    pub fn dir_set_csum(&self, dst_blk: &mut Block, dir_ino: u32, ino_gen: u32) {
        // The checksum covers the directory's own inode; only block 0 starts with its "." entry.
        let mut parent_de = Ext4DirEntry::default();
        parent_de.inode = dir_ino;

        let tail_offset = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
        let mut tail: Ext4DirEntryTail = *dst_blk.read_offset_as_mut(tail_offset);

        tail.tail_set_csum(&self.super_block, &parent_de, &dst_blk.data[..], ino_gen);

        tail.copy_to_slice(&mut dst_blk.data);
    }

    /// Add a new entry to a directory
    ///
    /// Params:
    /// parent: &mut Ext4InodeRef - parent directory inode reference
    /// child: &mut Ext4InodeRef - child inode reference
    /// path: &str - path of the new entry
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn dir_add_entry(
        &self,
        parent: &mut Ext4InodeRef,
        child: &Ext4InodeRef,
        name: &str,
    ) -> Result<usize> {
        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let block_size = self.super_block.block_size();
        let total_blocks: u64 = inode_size / block_size as u64;
        let de_type = DirEntryType::from_inode(&child.inode);

        // iterate all blocks
        let mut iblock = 0;
        while iblock < total_blocks {
            // get physical block id of a logical block id
            let pblock = self.get_pblock_idx(parent, iblock as u32)?;

            // load physical block
            let mut ext4block =
                Block::load(&self.block_device, pblock as usize * BLOCK_SIZE);

            let result = self.try_insert_to_existing_block(&mut ext4block, name, child.inode_num, de_type);

            if let Err(e) = result {
                if e.error() != Errno::ENOSPC {
                    return Err(e);
                }
            } else {
                // set checksum
                self.dir_set_csum(&mut ext4block, parent.inode_num, parent.inode.generation());
                ext4block.sync_blk_to_disk(&self.block_device);

                return Ok(EOK);
            }

            // go ot next block
            iblock += 1;
        }

        // no space in existing blocks, need to add new block
        let new_block = self.append_inode_pblk(parent)?;

        // load new block
        let mut new_ext4block =
            Block::load(&self.block_device, new_block as usize * BLOCK_SIZE);

        // write new entry to the new block
        // must succeed, as we just allocated the block
        self.insert_to_new_block(&mut new_ext4block, child.inode_num, name, de_type);

        // set checksum
        self.dir_set_csum(&mut new_ext4block, parent.inode_num, parent.inode.generation());
        new_ext4block.sync_blk_to_disk(&self.block_device);

        Ok(EOK)
    }

    /// Try to insert a new entry to an existing block
    ///
    /// Params:
    /// block: &mut Block - block to insert the new entry
    /// name: &str - name of the new entry
    /// inode: u32 - inode number of the new entry
    ///
    /// Returns:
    /// `Result<usize>` - status of the operation
    pub fn try_insert_to_existing_block(
        &self,
        block: &mut Block,
        name: &str,
        child_inode: u32,
        de_type: DirEntryType,
    ) -> Result<usize> {
        // required length aligned to 4 bytes
        let required_len = {
            let mut len = size_of::<Ext4DirEntry>() + name.len();
            if len % 4 != 0 {
                len += 4 - (len % 4);
            }
            len
        };

        let mut offset = 0;

        while offset < BLOCK_SIZE - size_of::<Ext4DirEntryTail>() {
            let (inode, rec_len, name_len) = dir_entry_header(&block.data, offset)?;
            let used = (size_of::<Ext4FakeDirEntry>() + name_len + 3) & !3;

            // A deleted entry (inode 0) is skipped, not reused.
            if inode != 0 && rec_len - used >= required_len {
                let free_space = rec_len - used;
                let mut new_entry = Ext4DirEntry::default();
                new_entry.write_entry(free_space as u16, child_inode, name, de_type);

                block.data[offset + 4..offset + 6].copy_from_slice(&(used as u16).to_le_bytes());
                new_entry.copy_to_slice(&mut block.data, offset + used);

                block.sync_blk_to_disk(&self.block_device);

                return Ok(EOK);
            }

            offset += rec_len;
        }

        return_errno_with_message!(Errno::ENOSPC, "No space in block for new entry");
    }

    /// Insert a new entry to a new block
    ///
    /// Params:
    /// block: &mut Block - block to insert the new entry
    /// name: &str - name of the new entry
    /// inode: u32 - inode number of the new entry
    pub fn insert_to_new_block(
        &self,
        block: &mut Block,
        inode: u32,
        name: &str,
        de_type: DirEntryType,
    ) {
        // write new entry
        let mut new_entry = Ext4DirEntry::default();
        let el = BLOCK_SIZE - size_of::<Ext4DirEntryTail>();
        new_entry.write_entry(el as u16, inode, name, de_type);
        new_entry.copy_to_slice(&mut block.data, 0);

        copy_dir_entry_to_array(&new_entry, &mut block.data, 0);

        // init tail for new block
        let tail = Ext4DirEntryTail::new();
        tail.copy_to_slice(&mut block.data);
    }

    pub fn dir_remove_entry(&self, parent: &mut Ext4InodeRef, path: &str) -> Result<usize> {
        // get remove_entry pos in parent and its prev entry
        let mut result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        let r = self.dir_find_entry(parent.inode_num, path, &mut result)?;

        let mut ext4block = Block::load(&self.block_device, result.pblock_id * BLOCK_SIZE);

        // Invalidate entry first
        let de_del: &mut Ext4DirEntry = ext4block.read_offset_as_mut(result.offset);
        de_del.inode = 0;

        // Store entry position in block
        let pos = result.offset;

        // If entry is not the first in block, it must be merged with previous entry
        if pos != 0 {
            let mut offset = 0;

            // Start from the first entry in block
            let mut tmp_de: Ext4DirEntry = ext4block.read_offset_as(offset);
            let mut de_len = tmp_de.entry_len();

            // Find direct predecessor of removed entry
            while (offset + de_len as usize) < pos {
                offset += de_len as usize;
                tmp_de = ext4block.read_offset_as(offset);
                de_len = tmp_de.entry_len();
            }
            
            assert!(de_len as usize + offset == pos, "Invalid predecessor calculation");

            // Add removed entry length to predecessor's length
            let del_len = result.dentry.entry_len();
            let mut tmp_de_mut: &mut Ext4DirEntry = ext4block.read_offset_as_mut(offset);
            tmp_de_mut.entry_len = de_len + del_len;
        }

        self.dir_set_csum(&mut ext4block, parent.inode_num, parent.inode.generation());
        ext4block.sync_blk_to_disk(&self.block_device);

        Ok(EOK)
    }

    pub fn dir_has_entry(&self, dir_inode: u32) -> bool {
        // load parent inode
        let parent = self.get_inode_ref(dir_inode);
        assert!(parent.inode.is_dir());

        // start from the first logical block
        let mut iblock = 0;
        // physical block id
        let mut fblock: Ext4Fsblk = 0;

        // calculate total blocks
        let inode_size: u64 = parent.inode.size();
        let total_blocks: u64 = inode_size / BLOCK_SIZE as u64;

        // iterate all blocks
        while iblock < total_blocks {
            let search_path = self.find_extent(&parent, iblock as u32);

            if let Ok(path) = search_path {
                // get the last path
                let path = path.path.last().unwrap();

                // get physical block id
                fblock = path.pblock;

                // load physical block
                let ext4block =
                    Block::load(&self.block_device, fblock as usize * BLOCK_SIZE);

                // start from the first entry
                let mut offset = 0;
                while offset < BLOCK_SIZE - core::mem::size_of::<Ext4DirEntryTail>() {
                    let de: Ext4DirEntry = ext4block.read_offset_as(offset);
                    offset += de.entry_len as usize;
                    if de.inode == 0 {
                        continue;
                    }
                    // skip . and ..
                    if de.get_name() == "." || de.get_name() == ".." {
                        continue;
                    }
                    return true;
                }
            }
            // go to next block
            iblock += 1
        }

        false
    }

    pub fn dir_remove(&self, parent: u32, path: &str) -> Result<usize> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        let r = self.dir_find_entry(parent, path, &mut search_result)?;

        let mut parent_inode_ref = self.get_inode_ref(parent);
        let mut child_inode_ref = self.get_inode_ref(search_result.dentry.inode);

        if self.dir_has_entry(child_inode_ref.inode_num){
            return_errno_with_message!(Errno::ENOTSUP, "rm dir with children not supported")
        }
        
        self.truncate_inode(&mut child_inode_ref, 0)?;

        self.unlink(&mut parent_inode_ref, &mut child_inode_ref, path)?;

        self.write_back_inode(&mut parent_inode_ref);

        // to do
        // ext4_inode_set_del_time
        // ext4_inode_set_links_cnt
        // ext4_fs_free_inode(&child)

        Ok(EOK)
    }
}

pub fn copy_dir_entry_to_array(header: &Ext4DirEntry, array: &mut [u8], offset: usize) {
    unsafe {
        let de_ptr = header as *const Ext4DirEntry as *const u8;
        let array_ptr = array as *mut [u8] as *mut u8;
        let count = core::mem::size_of::<Ext4DirEntry>() / core::mem::size_of::<u8>();
        core::ptr::copy_nonoverlapping(de_ptr, array_ptr.add(offset), count);
    }
}

/// (inode, rec_len, name_len) of the entry at `offset`, refusing a record that loops or leaves the block.
fn dir_entry_header(data: &[u8], offset: usize) -> Result<(u32, usize, usize)> {
    let header_len = size_of::<Ext4FakeDirEntry>();
    if offset + header_len > BLOCK_SIZE {
        return_errno_with_message!(Errno::EIO, "Corrupt directory entry");
    }
    let inode = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    let rec_len = u16::from_le_bytes(data[offset + 4..offset + 6].try_into().unwrap()) as usize;
    let name_len = data[offset + 6] as usize;
    if rec_len < header_len
        || rec_len % 4 != 0
        || offset + rec_len > BLOCK_SIZE
        || header_len + name_len > rec_len
    {
        return_errno_with_message!(Errno::EIO, "Corrupt directory entry");
    }
    Ok((inode, rec_len, name_len))
}
