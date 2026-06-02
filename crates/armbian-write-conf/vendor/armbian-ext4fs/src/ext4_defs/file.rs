use core::default;

use super::*;

pub struct FileAttr {
    /// Inode number
    pub ino: u64,
    /// Size in bytes
    pub size: u64,
    /// Size in blocks
    pub blocks: u64,
    /// Time of last access
    pub atime: u32,
    /// Time of last modification
    pub mtime: u32,
    /// Time of last change
    pub ctime: u32,
    /// Time of creation (macOS only)
    pub crtime: u32,
    /// Time of last status change
    pub chgtime: u32,
    /// Backup time (macOS only)
    pub bkuptime: u32,
    /// Kind of file (directory, file, pipe, etc)
    pub kind: InodeFileType,
    /// Permissions
    pub perm: InodePerm,
    /// Number of hard links
    pub nlink: u32,
    /// User id
    pub uid: u32,
    /// Group id
    pub gid: u32,
    /// Rdev
    pub rdev: u32,
    /// Block size
    pub blksize: u32,
    /// Flags (macOS only, see chflags(2))
    pub flags: u32,
}

impl Default for FileAttr {
    fn default() -> Self {
        FileAttr {
            ino: 0,
            size: 0,
            blocks: 0,
            atime: 0,
            mtime: 0,
            ctime: 0,
            crtime: 0,
            chgtime: 0,
            bkuptime: 0,
            kind: InodeFileType::S_IFREG,
            perm: InodePerm::S_IREAD | InodePerm::S_IWRITE | InodePerm::S_IEXEC,
            nlink: 0,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: 0,
            flags: 0,
        }
    }
}

impl FileAttr {
    pub fn from_inode_ref(inode_ref: &Ext4InodeRef) -> FileAttr {
        let inode_num = inode_ref.inode_num;
        let inode = inode_ref.inode;
        FileAttr {
            ino: inode_num as u64,
            size: inode.size(),
            blocks: inode.blocks_count(),
            atime: inode.atime(),
            mtime: inode.mtime(),
            ctime: inode.ctime(),
            crtime: inode.i_crtime(),
            // todo: chgtime, bkuptime
            chgtime: 0,
            bkuptime: 0,
            kind: inode.file_type(),
            perm: inode.file_perm(), // Extract permission bits
            nlink: inode.links_count() as u32,
            uid: inode.uid() as u32,
            gid: inode.gid() as u32,
            rdev: inode.faddr(),
            blksize: BLOCK_SIZE as u32,
            flags: inode.flags(),
        }
    }
}

// #ifdef __i386__
// struct stat {
// 	unsigned long  st_dev;
// 	unsigned long  st_ino;
// 	unsigned short st_mode;
// 	unsigned short st_nlink;
// 	unsigned short st_uid;
// 	unsigned short st_gid;
// 	unsigned long  st_rdev;
// 	unsigned long  st_size;
// 	unsigned long  st_blksize;
// 	unsigned long  st_blocks;
// 	unsigned long  st_atime;
// 	unsigned long  st_atime_nsec;
// 	unsigned long  st_mtime;
// 	unsigned long  st_mtime_nsec;
// 	unsigned long  st_ctime;
// 	unsigned long  st_ctime_nsec;
// 	unsigned long  __unused4;
// 	unsigned long  __unused5;
// };

#[repr(C)]
pub struct LinuxStat {
    st_dev: u32,        // ID of device containing file
    st_ino: u32,        // Inode number
    st_mode: u16,       // File type and mode
    st_nlink: u16,      // Number of hard links
    st_uid: u16,        // User ID of owner
    st_gid: u16,        // Group ID of owner
    st_rdev: u32,       // Device ID (if special file)
    st_size: u32,       // Total size, in bytes
    st_blksize: u32,    // Block size for filesystem I/O
    st_blocks: u32,     // Number of 512B blocks allocated
    st_atime: u32,      // Time of last access
    st_atime_nsec: u32, // Nanoseconds part of last access time
    st_mtime: u32,      // Time of last modification
    st_mtime_nsec: u32, // Nanoseconds part of last modification time
    st_ctime: u32,      // Time of last status change
    st_ctime_nsec: u32, // Nanoseconds part of last status change time
    __unused4: u32,     // Unused field
    __unused5: u32,     // Unused field
}

impl LinuxStat {
    pub fn from_inode_ref(inode_ref: &Ext4InodeRef) -> LinuxStat {
        let inode_num = inode_ref.inode_num;
        let inode = &inode_ref.inode;

        LinuxStat {
            st_dev: 0,
            st_ino: inode_num,
            st_mode: inode.mode,
            st_nlink: inode.links_count(),
            st_uid: inode.uid(),
            st_gid: inode.gid(),
            st_rdev: 0,
            st_size: inode.size() as u32,
            st_blksize: 4096, // Block size assumed to be 4096 bytes
            st_blocks: inode.blocks_count() as u32,
            st_atime: inode.atime(),
            st_atime_nsec: 0,
            st_mtime: inode.mtime(),
            st_mtime_nsec: 0,
            st_ctime: inode.ctime(),
            st_ctime_nsec: 0,
            __unused4: 0,
            __unused5: 0,
        }
    }
}
