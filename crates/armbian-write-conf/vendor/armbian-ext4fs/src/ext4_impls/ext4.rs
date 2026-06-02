use crate::prelude::*;
use crate::return_errno_with_message;
use crate::utils::*;

use crate::ext4_defs::*;

impl Ext4 {
    /// 获取system zone缓存
    pub fn get_system_zone(&self) -> Vec<SystemZone> {
        let mut zones = Vec::new();
        let group_count = self.super_block.block_group_count();
        let inodes_per_group = self.super_block.inodes_per_group();
        let inode_size = self.super_block.inode_size() as u64;
        let block_size = self.super_block.block_size() as u64;
        for bgid in 0..group_count {
            // meta blocks
            let meta_blks = self.num_base_meta_blocks(bgid);
            if meta_blks != 0 {
                let start = self.get_block_of_bgid(bgid);
                zones.push(SystemZone {
                    group: bgid,
                    start_blk: start,
                    end_blk: start + meta_blks as u64 - 1,
                });
            }
            // block group描述符
            let block_group = Ext4BlockGroup::load_new(&self.block_device, &self.super_block, bgid as usize);
            // block bitmap
            let blk_bmp = block_group.get_block_bitmap_block(&self.super_block);
            zones.push(SystemZone {
                group: bgid,
                start_blk: blk_bmp,
                end_blk: blk_bmp,
            });
            // inode bitmap
            let ino_bmp = block_group.get_inode_bitmap_block(&self.super_block);
            zones.push(SystemZone {
                group: bgid,
                start_blk: ino_bmp,
                end_blk: ino_bmp,
            });
            // inode table
            let ino_tbl = block_group.get_inode_table_blk_num() as u64;
            let itb_per_group = ((inodes_per_group as u64 * inode_size + block_size - 1) / block_size) as u64;
            zones.push(SystemZone {
                group: bgid,
                start_blk: ino_tbl,
                end_blk: ino_tbl + itb_per_group - 1,
            });
        }
        zones
    }
    /// Opens and loads an Ext4 from the `block_device`.
    pub fn open(block_device: Arc<dyn BlockDevice>) -> Self {
        // Load the superblock
        let block = Block::load(&block_device, SUPERBLOCK_OFFSET);
        let super_block: Ext4Superblock = block.read_as();

        // drop(block);
        
        let ext4_tmp = Ext4 {
            block_device,
            super_block,
            system_zone_cache: None,
        };
        let zones = ext4_tmp.get_system_zone();

        Ext4 {
            system_zone_cache: Some(zones),
            ..ext4_tmp
        }
    }

    // with dir result search path offset
    pub fn generic_open(
        &self,
        path: &str,
        parent_inode_num: &mut u32,
        create: bool,
        ftype: u16,
        name_off: &mut u32,
    ) -> Result<u32> {
        let mut is_goal = false;

        let mut parent = parent_inode_num;

        let mut search_path = path;

        let mut dir_search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        loop {
            while search_path.starts_with('/') {
                *name_off += 1; // Skip the slash
                search_path = &search_path[1..];
            }

            let len = path_check(search_path, &mut is_goal);

            let current_path = &search_path[..len];

            if len == 0 || search_path.is_empty() {
                break;
            }

            search_path = &search_path[len..];

            let r = self.dir_find_entry(*parent, current_path, &mut dir_search_result);

            // log::trace!("find in parent {:x?} r {:?} name {:?}", parent, r, current_path);
            if let Err(e) = r {
                if e.error() != Errno::ENOENT || !create {
                    return_errno_with_message!(Errno::ENOENT, "No such file or directory");
                }

                let mut inode_mode = 0;
                if is_goal {
                    inode_mode = ftype;
                } else {
                    inode_mode = InodeFileType::S_IFDIR.bits();
                }

                let new_inode_ref = self.create(*parent, current_path, inode_mode)?;

                // Update parent to the new inode
                *parent = new_inode_ref.inode_num;

                // Now, update dir_search_result to reflect the new inode
                dir_search_result.dentry.inode = new_inode_ref.inode_num;

                continue;
            }

            if is_goal {
                break;
            } else {
                // update parent
                *parent = dir_search_result.dentry.inode;
            }
            *name_off += len as u32;
        }

        if is_goal {
            return Ok(dir_search_result.dentry.inode);
        }

        Ok(dir_search_result.dentry.inode)
    }

    #[allow(unused)]
    pub fn dir_mk(&self, path: &str) -> Result<usize> {
        let mut nameoff = 0;

        let filetype = InodeFileType::S_IFDIR;

        // todo get this path's parent

        // start from root
        let mut parent = ROOT_INODE;

        let r = self.generic_open(path, &mut parent, true, filetype.bits(), &mut nameoff);
        Ok(EOK)
    }

    pub fn unlink(
        &self,
        parent: &mut Ext4InodeRef,
        child: &mut Ext4InodeRef,
        name: &str,
    ) -> Result<usize> {
        self.dir_remove_entry(parent, name)?;

        let is_dir = child.inode.is_dir();

        self.ialloc_free_inode(child.inode_num, is_dir);

        Ok(EOK)
    }
}
