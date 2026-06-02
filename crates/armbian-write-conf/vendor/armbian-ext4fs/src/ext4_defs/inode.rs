use crate::prelude::*;
use crate::utils::*;

use super::*;

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Ext4Inode {
    pub mode: u16,        // File type and permissions
    pub uid: u16,         // Owner user ID
    pub size: u32,        // Lower 32 bits of file size
    pub atime: u32,       // Last access time
    pub ctime: u32,       // Creation time
    pub mtime: u32,       // Last modification time
    pub dtime: u32,       // Deletion time
    pub gid: u16,         // Owner group ID
    pub links_count: u16, // Link count
    pub blocks: u32,      // Allocated blocks count
    pub flags: u32,       // File flags
    pub osd1: u32,        // OS-dependent field 1
    pub block: [u32; 15], // Data block pointers
    pub generation: u32,  // File version (NFS)
    pub file_acl: u32,    // File ACL
    pub size_hi: u32,     // Higher 32 bits of file size
    pub faddr: u32,       // Deprecated fragment address
    pub osd2: Linux2,     // OS-dependent field 2

    pub i_extra_isize: u16,  // Extra inode size
    pub i_checksum_hi: u16,  // High checksum (crc32c(uuid+inum+inode) BE)
    pub i_ctime_extra: u32,  // Extra creation time (nanosec << 2 | epoch)
    pub i_mtime_extra: u32,  // Extra modification time (nanosec << 2 | epoch)
    pub i_atime_extra: u32,  // Extra access time (nanosec << 2 | epoch)
    pub i_crtime: u32,       // Creation time
    pub i_crtime_extra: u32, // Extra creation time (nanosec << 2 | epoch)
    pub i_version_hi: u32,   // Higher 32 bits of version
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Linux2 {
    pub l_i_blocks_high: u16,   // Higher 16 bits of allocated blocks count
    pub l_i_file_acl_high: u16, // Higher 16 bits of file ACL
    pub l_i_uid_high: u16,      // Higher 16 bits of user ID
    pub l_i_gid_high: u16,      // Higher 16 bits of group ID
    pub l_i_checksum_lo: u16,   // Lower checksum
    pub l_i_reserved: u16,      // Reserved field
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodeFileType: u16 {
        const S_IFIFO = 0x1000;
        const S_IFCHR = 0x2000;
        const S_IFDIR = 0x4000;
        const S_IFBLK = 0x6000;
        const S_IFREG = 0x8000;
        const S_IFSOCK = 0xC000;
        const S_IFLNK = 0xA000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct InodePerm: u16 {
        const S_IREAD = 0x0100;
        const S_IWRITE = 0x0080;
        const S_IEXEC = 0x0040;
        const S_ISUID = 0x0800;
        const S_ISGID = 0x0400;
    }
}

impl Ext4Inode {
    pub fn mode(&self) -> u16 {
        self.mode
    }

    pub fn set_mode(&mut self, mode: u16) {
        self.mode = mode;
    }

    pub fn uid(&self) -> u16 {
        self.uid
    }

    pub fn set_uid(&mut self, uid: u16) {
        self.uid = uid;
    }

    pub fn size(&self) -> u64 {
        self.size as u64 | ((self.size_hi as u64) << 32)
    }

    pub fn set_size(&mut self, size: u64) {
        self.size = (size & 0xffffffff) as u32;
        self.size_hi = (size >> 32) as u32;
    }

    pub fn atime(&self) -> u32 {
        self.atime
    }

    pub fn set_atime(&mut self, atime: u32) {
        self.atime = atime;
    }

    pub fn ctime(&self) -> u32 {
        self.ctime
    }

    pub fn set_ctime(&mut self, ctime: u32) {
        self.ctime = ctime;
    }

    pub fn mtime(&self) -> u32 {
        self.mtime
    }

    pub fn set_mtime(&mut self, mtime: u32) {
        self.mtime = mtime;
    }

    pub fn dtime(&self) -> u32 {
        self.dtime
    }

    pub fn set_dtime(&mut self, dtime: u32) {
        self.dtime = dtime;
    }

    pub fn gid(&self) -> u16 {
        self.gid
    }

    pub fn set_gid(&mut self, gid: u16) {
        self.gid = gid;
    }

    pub fn links_count(&self) -> u16 {
        self.links_count
    }

    pub fn set_links_count(&mut self, links_count: u16) {
        self.links_count = links_count;
    }

    pub fn blocks_count(&self) -> u64 {
        let mut blocks = self.blocks as u64;
        if self.osd2.l_i_blocks_high != 0 {
            blocks |= (self.osd2.l_i_blocks_high as u64) << 32;
        }
        blocks
    }

    pub fn set_blocks_count(&mut self, blocks: u64) {
        self.blocks = (blocks & 0xFFFFFFFF) as u32;
        self.osd2.l_i_blocks_high = (blocks >> 32) as u16;
    }

    pub fn flags(&self) -> u32 {
        self.flags
    }

    pub fn set_flags(&mut self, flags: u32) {
        self.flags = flags;
    }

    pub fn osd1(&self) -> u32 {
        self.osd1
    }

    pub fn set_osd1(&mut self, osd1: u32) {
        self.osd1 = osd1;
    }

    pub fn block(&self) -> [u32; 15] {
        self.block
    }

    pub fn set_block(&mut self, block: [u32; 15]) {
        self.block = block;
    }

    pub fn generation(&self) -> u32 {
        self.generation
    }

    pub fn set_generation(&mut self, generation: u32) {
        self.generation = generation;
    }

    pub fn file_acl(&self) -> u32 {
        self.file_acl
    }

    pub fn set_file_acl(&mut self, file_acl: u32) {
        self.file_acl = file_acl;
    }

    pub fn size_hi(&self) -> u32 {
        self.size_hi
    }

    pub fn set_size_hi(&mut self, size_hi: u32) {
        self.size_hi = size_hi;
    }

    pub fn faddr(&self) -> u32 {
        self.faddr
    }

    pub fn set_faddr(&mut self, faddr: u32) {
        self.faddr = faddr;
    }

    pub fn osd2(&self) -> Linux2 {
        self.osd2
    }

    pub fn set_osd2(&mut self, osd2: Linux2) {
        self.osd2 = osd2;
    }

    pub fn i_extra_isize(&self) -> u16 {
        self.i_extra_isize
    }

    pub fn set_i_extra_isize(&mut self, i_extra_isize: u16) {
        self.i_extra_isize = i_extra_isize;
    }

    pub fn i_checksum_hi(&self) -> u16 {
        self.i_checksum_hi
    }

    pub fn set_i_checksum_hi(&mut self, i_checksum_hi: u16) {
        self.i_checksum_hi = i_checksum_hi;
    }

    pub fn i_ctime_extra(&self) -> u32 {
        self.i_ctime_extra
    }

    pub fn set_i_ctime_extra(&mut self, i_ctime_extra: u32) {
        self.i_ctime_extra = i_ctime_extra;
    }

    pub fn i_mtime_extra(&self) -> u32 {
        self.i_mtime_extra
    }

    pub fn set_i_mtime_extra(&mut self, i_mtime_extra: u32) {
        self.i_mtime_extra = i_mtime_extra;
    }

    pub fn i_atime_extra(&self) -> u32 {
        self.i_atime_extra
    }

    pub fn set_i_atime_extra(&mut self, i_atime_extra: u32) {
        self.i_atime_extra = i_atime_extra;
    }

    pub fn i_crtime(&self) -> u32 {
        self.i_crtime
    }

    pub fn set_i_crtime(&mut self, i_crtime: u32) {
        self.i_crtime = i_crtime;
    }

    pub fn i_crtime_extra(&self) -> u32 {
        self.i_crtime_extra
    }

    pub fn set_i_crtime_extra(&mut self, i_crtime_extra: u32) {
        self.i_crtime_extra = i_crtime_extra;
    }

    pub fn i_version_hi(&self) -> u32 {
        self.i_version_hi
    }

    pub fn set_i_version_hi(&mut self, i_version_hi: u32) {
        self.i_version_hi = i_version_hi;
    }
}

impl Ext4Inode {
    pub fn file_type(&self) -> InodeFileType {
        InodeFileType::from_bits_truncate(self.mode & EXT4_INODE_MODE_TYPE_MASK)
    }

    pub fn file_perm(&self) -> InodePerm {
        InodePerm::from_bits_truncate(self.mode & EXT4_INODE_MODE_PERM_MASK)
    }

    pub fn is_dir(&self) -> bool {
        self.file_type() == InodeFileType::S_IFDIR
    }

    pub fn is_file(&self) -> bool {
        self.file_type() == InodeFileType::S_IFREG
    }

    pub fn is_link(&self) -> bool {
        self.file_type() == InodeFileType::S_IFLNK
    }

    pub fn can_read(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IREAD)
    }

    pub fn can_write(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IWRITE)
    }

    pub fn can_exec(&self) -> bool {
        self.file_perm().contains(InodePerm::S_IEXEC)
    }

    pub fn set_file_type(&mut self, kind: InodeFileType) {
        self.mode |= kind.bits();
    }

    pub fn set_file_perm(&mut self, perm: InodePerm) {
        self.mode |= perm.bits();
    }
}

/// Reference to an inode.
#[derive(Clone)]
pub struct Ext4InodeRef {
    pub inode_num: u32,
    pub inode: Ext4Inode,
}

impl Ext4Inode {
    /// Get the depth of the extent tree from an inode.
    pub fn root_header_depth(&self) -> u16 {
        self.root_extent_header().depth
    }

    pub fn root_extent_header_ref(&self) -> &Ext4ExtentHeader {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { &*header_ptr }
    }

    pub fn root_extent_header(&self) -> Ext4ExtentHeader {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { *header_ptr }
    }

    pub fn root_extent_header_mut(&mut self) -> &mut Ext4ExtentHeader {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *header_ptr }
    }

    pub fn root_extent_mut_at(&mut self, pos: usize) -> &mut Ext4Extent {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *(header_ptr.add(1) as *mut Ext4Extent).add(pos) }
    }

    pub fn root_extent_ref_at(&mut self, pos: usize) -> &Ext4Extent {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { &*(header_ptr.add(1) as *const Ext4Extent).add(pos) }
    }

    pub fn root_extent_at(&mut self, pos: usize) -> Ext4Extent {
        let header_ptr = self.block.as_ptr() as *const Ext4ExtentHeader;
        unsafe { *(header_ptr.add(1) as *const Ext4Extent).add(pos) }
    }

    pub fn root_first_index_mut(&mut self) -> &mut Ext4ExtentIndex {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe { &mut *(header_ptr.add(1) as *mut Ext4ExtentIndex) }
    }

    pub fn extent_tree_init(&mut self) {
        let header_ptr = self.block.as_mut_ptr() as *mut Ext4ExtentHeader;
        unsafe {
            (*header_ptr).set_magic();
            (*header_ptr).set_entries_count(0);
            (*header_ptr).set_max_entries_count(4);
            (*header_ptr).set_depth(0);
            (*header_ptr).set_generation(0);
        }
    }

    fn get_checksum(&self, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size;
        let mut v: u32 = self.osd2.l_i_checksum_lo as u32;
        if inode_size > 128 {
            v |= (self.i_checksum_hi as u32) << 16;
        }
        v
    }
    #[allow(unused)]
    pub fn set_inode_checksum_value(
        &mut self,
        super_block: &Ext4Superblock,
        inode_id: u32,
        checksum: u32,
    ) {
        let inode_size = super_block.inode_size();

        self.osd2.l_i_checksum_lo = (checksum & 0xffff) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }
    fn copy_to_slice(&self, slice: &mut [u8]) {
        unsafe {
            let inode_ptr = self as *const Ext4Inode as *const u8;
            let array_ptr = slice.as_ptr() as *mut u8;
            core::ptr::copy_nonoverlapping(inode_ptr, array_ptr, 0x9c);
        }
    }
    #[allow(unused)]
    pub fn get_inode_checksum(&mut self, inode_id: u32, super_block: &Ext4Superblock) -> u32 {
        let inode_size = super_block.inode_size();

        let orig_checksum = self.get_checksum(super_block);
        let mut checksum = 0;

        let ino_index = inode_id;
        let ino_gen = self.generation;

        // Preparation: temporarily set bg checksum to 0
        self.osd2.l_i_checksum_lo = 0;
        self.i_checksum_hi = 0;

        checksum = ext4_crc32c(
            EXT4_CRC32_INIT,
            &super_block.uuid,
            super_block.uuid.len() as u32,
        );
        checksum = ext4_crc32c(checksum, &ino_index.to_le_bytes(), 4);
        checksum = ext4_crc32c(checksum, &ino_gen.to_le_bytes(), 4);

        let mut raw_data = [0u8; 0x100];
        self.copy_to_slice(&mut raw_data);

        // inode checksum
        checksum = ext4_crc32c(checksum, &raw_data, inode_size as u32);

        self.set_inode_checksum_value(super_block, inode_id, checksum);

        if inode_size == 128 {
            checksum &= 0xFFFF;
        }

        checksum
    }

    pub fn set_inode_checksum(&mut self, super_block: &Ext4Superblock, inode_id: u32) {
        let inode_size = super_block.inode_size();
        let checksum = self.get_inode_checksum(inode_id, super_block);

        self.osd2.l_i_checksum_lo = ((checksum << 16) >> 16) as u16;
        if inode_size > 128 {
            self.i_checksum_hi = (checksum >> 16) as u16;
        }
    }

    pub fn sync_inode_to_disk(&self, block_device: &Arc<dyn BlockDevice>, inode_pos: usize) {
        let data = unsafe {
            core::slice::from_raw_parts(self as *const _ as *const u8, size_of::<Ext4Inode>())
        };
        block_device.write_offset(inode_pos, data);
    }

    pub fn root_extent_block(&self) -> u64 {
        let block_lo = self.block[0] as u64;
        let block_hi = self.block[1] as u64;
        block_lo | (block_hi << 32)
    }
}

impl Ext4InodeRef {
    pub fn set_attr(&mut self, attr: &FileAttr) {
        self.inode.set_size(attr.size);
        self.inode.set_blocks_count(attr.blocks);
        self.inode.set_atime(attr.atime);
        self.inode.set_mtime(attr.mtime);
        self.inode.set_ctime(attr.ctime);
        self.inode.set_i_crtime(attr.crtime);
        self.inode.set_file_type(attr.kind);
        self.inode.set_file_perm(attr.perm);
        self.inode.set_links_count(attr.nlink as u16);
        self.inode.set_uid(attr.uid as u16);
        self.inode.set_gid(attr.gid as u16);
        self.inode.set_faddr(attr.rdev);
        self.inode.set_flags(attr.flags);
    }
}

impl Ext4Inode {
    //    access() does not answer the "can I read/write/execute
    //    this file?" question.  It answers a slightly different question:
    //    "(assuming I'm a setuid binary) can the user who invoked me
    //    read/write/execute this file?", which gives set-user-ID programs
    //    the possibility to prevent malicious users from causing them to
    //    read files which users shouldn't be able to read.
    //   https://man7.org/linux/man-pages/man2/access.2.html
    // Check if a user can access the inode with the given UID, GID, and umask
    pub fn check_access(&self, uid: u16, gid: u16, access_mode: u16, umask: u16) -> bool {
        // Extract the owner, group, and other permission bits from the inode's mode
        let owner_perm = (self.mode & 0o700) >> 6;
        let group_perm = (self.mode & 0o070) >> 3;
        let other_perm = self.mode & 0o007;

        // Determine which permission bits to check based on the given UID and GID
        let perm = if self.uid == uid {
            owner_perm
        } else if self.gid == gid {
            group_perm
        } else {
            other_perm
        };

        // Adjust the permission bits based on the umask
        let adjusted_perm = perm & !((umask & 0o700) >> 6);

        // Check if the adjusted permission bits allow the requested access
        let check_read =
            (access_mode & R_OK as u16) == 0 || (adjusted_perm & R_OK as u16) == R_OK as u16;
        let check_write =
            (access_mode & W_OK as u16) == 0 || (adjusted_perm & W_OK as u16) == W_OK as u16;
        let check_execute =
            (access_mode & X_OK as u16) == 0 || (adjusted_perm & X_OK as u16) == X_OK as u16;

        check_read && check_write && check_execute
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_access_owner() {
        let inode = Ext4Inode {
            mode: 0o755, // rwxr-xr-x
            uid: 1000,
            gid: 1000,
            ..Default::default()
        };

        let uid = 1000;
        let gid = 1000;
        let umask = 0o022; // Default umask
        let access_mode = R_OK | X_OK;

        assert!(inode.check_access(uid, gid, umask, access_mode as u16));
    }

    #[test]
    fn test_check_access_group() {
        let inode = Ext4Inode {
            mode: 0o750, // rwxr-x---
            uid: 1000,
            gid: 1001,
            ..Default::default()
        };

        let uid = 1002;
        let gid = 1001;
        let umask = 0o022; // Default umask
        let access_mode = R_OK | X_OK;

        assert!(inode.check_access(uid, gid, access_mode as u16, umask));
    }

    #[test]
    fn test_check_access_other() {
        let inode = Ext4Inode {
            mode: 0o755, // rwxr-xr-x
            uid: 1000,
            gid: 1000,
            ..Default::default()
        };

        let uid = 1002;
        let gid = 1003;
        let umask = 0o022; // Default umask
        let access_mode = R_OK;

        assert!(inode.check_access(uid, gid, access_mode as u16, umask));
    }

    #[test]
    fn test_check_access_denied() {
        let inode = Ext4Inode {
            mode: 0o700, // rwx------
            uid: 1000,
            gid: 1000,
            ..Default::default()
        };

        let uid = 1002;
        let gid = 1003;
        let umask = 0o022; // Default umask
        let access_mode = R_OK;

        assert!(!inode.check_access(uid, gid, access_mode as u16, umask));
    }
    
    #[test]
    fn test_file_type() {
        let inode = Ext4Inode {
            mode: 0x8000, // Regular file
            ..Default::default()
        };
        assert!(inode.is_file());
        assert!(!inode.is_dir());
        assert!(!inode.is_link());
    }

    #[test]
    fn test_file_permissions() {
        let inode = Ext4Inode {
            mode: 0o755, // rwxr-xr-x
            ..Default::default()
        };
        assert!(inode.can_read());
        assert!(inode.can_write());
        assert!(inode.can_exec());
    }

    #[test]
    fn test_set_file_type_and_perm() {
        let mut inode = Ext4Inode {
            mode: 0,
            ..Default::default()
        };
        inode.set_file_type(InodeFileType::S_IFREG);
        assert_eq!(inode.mode, InodeFileType::S_IFREG.bits()); // Regular file with rwx permissions
        inode.set_file_perm(InodePerm::S_IREAD | InodePerm::S_IWRITE | InodePerm::S_IEXEC);
        assert_eq!(inode.mode, InodeFileType::S_IFREG.bits() | (InodePerm::S_IREAD | InodePerm::S_IWRITE | InodePerm::S_IEXEC).bits()); // Regular file with rwx permissions
    }
}
