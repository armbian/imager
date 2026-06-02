use crate::prelude::*;

use crate::ext4_defs::*;
use crate::return_errno;
use crate::return_errno_with_message;
use crate::utils::path_check;

// export some definitions
pub use crate::ext4_defs::Ext4;
pub use crate::ext4_defs::BLOCK_SIZE;
pub use crate::ext4_defs::BlockDevice;
pub use crate::ext4_defs::InodeFileType;

/// fuser interface for ext4
impl Ext4 {
    /// Look up a directory entry by name and get its attributes.
    pub fn fuse_lookup(&self, parent: u64, name: &str) -> Result<FileAttr> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        self.dir_find_entry(parent as u32, name, &mut search_result)?;

        let inode_num = search_result.dentry.inode;

        let inode_ref = self.get_inode_ref(inode_num);
        let file_attr = FileAttr::from_inode_ref(&inode_ref);

        Ok(file_attr)
    }

    /// Get file attributes.
    pub fn fuse_getattr(&self, ino: u64) -> Result<FileAttr> {
        let inode_ref = self.get_inode_ref(ino as u32);
        let file_attr = FileAttr::from_inode_ref(&inode_ref);
        Ok(file_attr)
    }

    /// Set file attributes.
    pub fn fuse_setattr(
        &self,
        ino: u64,
        mode: Option<u32>,
        uid: Option<u32>,
        gid: Option<u32>,
        size: Option<u64>,
        atime: Option<u32>,
        mtime: Option<u32>,
        ctime: Option<u32>,
        fh: Option<u64>,
        crtime: Option<u32>,
        chgtime: Option<u32>,
        bkuptime: Option<u32>,
        flags: Option<u32>,
    ) {
        let mut inode_ref = self.get_inode_ref(ino as u32);

        let mut attr = FileAttr::default();

        if let Some(mode) = mode {
            let inode_file_type =
                InodeFileType::from_bits(mode as u16 & EXT4_INODE_MODE_TYPE_MASK).unwrap();
            attr.kind = inode_file_type;
            let inode_perm = InodePerm::from_bits(mode as u16 & EXT4_INODE_MODE_PERM_MASK).unwrap();
            attr.perm = inode_perm;
        }

        if let Some(uid) = uid {
            attr.uid = uid
        }

        if let Some(gid) = gid {
            attr.gid = gid
        }

        if let Some(size) = size {
            attr.size = size
        }

        if let Some(atime) = atime {
            attr.atime = atime
        }

        if let Some(mtime) = mtime {
            attr.mtime = mtime
        }

        if let Some(ctime) = ctime {
            attr.ctime = ctime
        }

        if let Some(crtime) = crtime {
            attr.crtime = crtime
        }

        if let Some(chgtime) = chgtime {
            attr.chgtime = chgtime
        }

        if let Some(bkuptime) = bkuptime {
            attr.bkuptime = bkuptime
        }

        if let Some(flags) = flags {
            attr.flags = flags
        }

        inode_ref.set_attr(&attr);

        self.write_back_inode(&mut inode_ref);
    }

    /// Read symbolic link.
    fn fuse_readlink(&mut self, ino: u64) -> Result<Vec<u8>> {
        let inode_ref = self.get_inode_ref(ino as u32);
        let file_size = inode_ref.inode.size();
        let mut read_buf = vec![0; file_size as usize];
        let read_size = self.read_at(ino as u32, 0, &mut read_buf)?;
        Ok(read_buf)
    }


    /// Create a regular file, character device, block device, fifo or socket node.
    pub fn fuse_mknod(
        &self,
        parent: u64,
        name: &str,
        mode: u32,
        umask: u32,
        rdev: u32,
    ) -> Result<Ext4InodeRef> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, name, &mut search_result);
        if r.is_ok() {
            return_errno!(Errno::EEXIST);
        }
        let inode_ref = self.create(parent as u32, name, mode as u16)?;
        Ok(inode_ref)
    }


    /// Create a regular file, character device, block device, fifo or socket node.
    pub fn fuse_mknod_with_attr(
        &self,
        parent: u64,
        name: &str,
        mode: u32,
        umask: u32,
        rdev: u32,
        uid: u32, 
        gid: u32,
    ) -> Result<Ext4InodeRef> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, name, &mut search_result);
        if r.is_ok() {
            return_errno!(Errno::EEXIST);
        }
        let inode_ref = self.create_with_attr(parent as u32, name, mode as u16, uid as u16, gid as u16)?;
        Ok(inode_ref)
    }

    /// Create a directory.
    pub fn fuse_mkdir(&mut self, parent: u64, name: &str, mode: u32, umask: u32) -> Result<usize> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, name, &mut search_result);
        if r.is_ok() {
            return_errno!(Errno::EEXIST);
        }
        let file_type = InodeFileType::from_bits(mode as u16).unwrap();
        if file_type != InodeFileType::S_IFDIR {
            // The mode is not a directory
            return_errno_with_message!(Errno::EINVAL, "Invalid mode for directory creation");
        }
        let inode_ref = self.create(parent as u32, name, mode as u16)?;
        Ok(EOK)
    }

    /// Create a directory.
    pub fn fuse_mkdir_with_attr(&mut self, parent: u64, name: &str, mode: u32, umask: u32, uid:u32, gid:u32) -> Result<Ext4InodeRef> {

        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, name, &mut search_result);
        if r.is_ok() {
            return_errno!(Errno::EEXIST);
        }

        // mkdir via fuse passes a mode of 0. so we need to set default mode
        let file_type = match InodeFileType::from_bits(mode as u16) {
            Some(file_type) => file_type,
            None => InodeFileType::S_IFDIR,
        };
        let mode = file_type.bits();
        let inode_ref = self.create_with_attr(parent as u32, name, mode, uid as u16, gid as u16)?;

        Ok(inode_ref)
    }

    /// Remove a file.
    pub fn fuse_unlink(&self, parent: u64, name: &str) -> Result<usize> {
        // unlink actual remove a file

        // get child inode num
        let mut parent_inode = parent as u32;
        let mut nameoff = 0;
        let child_inode = self.generic_open(name, &mut parent_inode, false, 0, &mut nameoff)?;

        let mut child_inode_ref = self.get_inode_ref(child_inode);
        let child_link_cnt = child_inode_ref.inode.links_count();
        if child_link_cnt == 1 {
            self.truncate_inode(&mut child_inode_ref, 0)?;
        }

        // get child name
        let mut is_goal = false;
        let p = &name[nameoff as usize..];
        let len = path_check(p, &mut is_goal);

        // load parent
        let mut parent_inode_ref = self.get_inode_ref(parent_inode);

        let r = self.unlink(
            &mut parent_inode_ref,
            &mut child_inode_ref,
            &p[..len],
        )?;

        Ok(EOK)
    }
    /// Remove a directory.
    pub fn fuse_rmdir(&mut self, parent: u64, name: &str) -> Result<usize> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());

        let r = self.dir_find_entry(parent as u32, name, &mut search_result)?;

        let mut parent_inode_ref = self.get_inode_ref(parent as u32);
        let mut child_inode_ref = self.get_inode_ref(search_result.dentry.inode);

        self.truncate_inode(&mut child_inode_ref, 0)?;

        self.unlink(&mut parent_inode_ref, &mut child_inode_ref, name)?;

        self.write_back_inode(&mut parent_inode_ref);

        // to do
        // ext4_inode_set_del_time
        // ext4_inode_set_links_cnt
        // ext4_fs_free_inode(&child)

        Ok(EOK)
    }
    /// Create a symbolic link.
    pub fn fuse_symlink(&mut self, parent: u64, link_name: &str, target: &str) -> Result<usize> {
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, link_name, &mut search_result);
        if r.is_ok() {
            return_errno!(Errno::EEXIST);
        }

        let mut mode = 0o777;
        let file_type = InodeFileType::S_IFLNK;
        mode |= file_type.bits();

        let inode_ref = self.create(parent as u32, link_name, mode)?;
        Ok(EOK)
    }
    /// Create a hard link.
    /// Params:
    /// ino: the inode number of the source file
    /// newparent: the inode number of the new parent directory
    /// newname: the name of the new file
    ///
    ///
    pub fn fuse_link(&mut self, ino: u64, newparent: u64, newname: &str) -> Result<usize> {
        let mut parent_inode_ref = self.get_inode_ref(newparent as u32);
        let mut child_inode_ref = self.get_inode_ref(ino as u32);

        // to do if child already exists we should not add . and .. in child directory
        self.link(&mut parent_inode_ref, &mut child_inode_ref, newname)?;

        Ok(EOK)
    }

    /// Open a file.
    /// Open flags (with the exception of O_CREAT, O_EXCL, O_NOCTTY and O_TRUNC) are
    /// available in flags. Filesystem may store an arbitrary file handle (pointer, index,
    /// etc) in fh, and use this in other all other file operations (read, write, flush,
    /// release, fsync). Filesystem may also implement stateless file I/O and not store
    /// anything in fh. There are also some flags (direct_io, keep_cache) which the
    /// filesystem may set, to change the way the file is opened. See fuse_file_info
    /// structure in <fuse_common.h> for more details.
    pub fn fuse_open(&mut self, ino: u64, flags: i32) -> Result<usize> {
        let inode_ref = self.get_inode_ref(ino as u32);

        // check permission
        let file_type = inode_ref.inode.file_type();
        let file_perm = inode_ref.inode.file_perm();

        let can_read = file_perm.contains(InodePerm::S_IREAD);
        let can_write = file_perm.contains(InodePerm::S_IWRITE);
        let can_execute = file_perm.contains(InodePerm::S_IEXEC);

        // If trying to open the file in write mode, check for write permissions
        if ((flags & O_WRONLY != 0) || (flags & O_RDWR != 0)) && !can_write {
            return_errno_with_message!(Errno::EACCES, "Permission denied can not write");
        }
        // If trying to open the file in read mode, check for read permissions
        if ((flags & O_RDONLY != 0) || (flags & O_RDWR != 0)) && !can_read {
            return_errno_with_message!(Errno::EACCES, "Permission denied can not read");
        }

        // If trying to open the file in read mode, check for read permissions
        if ((flags & O_EXCL != 0) || (flags & O_RDWR != 0)) && !can_execute {
            return_errno_with_message!(Errno::EACCES, "Permission denied can not exec");
        }

        Ok(EOK)
    }

    /// Read data.
    /// Read should send exactly the number of bytes requested except on EOF or error,
    /// otherwise the rest of the data will be substituted with zeroes. An exception to
    /// this is when the file has been opened in 'direct_io' mode, in which case the
    /// return value of the read system call will reflect the return value of this
    /// operation. fh will contain the value set by the open method, or will be undefined
    /// if the open method didn't set any value.
    ///
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9
    /// lock_owner: only supported with ABI >= 7.9
    pub fn fuse_read(
        &self,
        ino: u64,
        fh: u64,
        offset: i64,
        size: u32,
        flags: i32,
        lock_owner: Option<u64>,
        ) -> Result<Vec<u8>> {
        let mut data = vec![0u8; size as usize];
        let read_size = self.read_at(ino as u32, offset as usize, &mut data)?;
        let r = data[..read_size].to_vec();
        Ok(r)
    }

    /// Write data.
    /// Write should return exactly the number of bytes requested except on error. An
    /// exception to this is when the file has been opened in 'direct_io' mode, in
    /// which case the return value of the write system call will reflect the return
    /// value of this operation. fh will contain the value set by the open method, or
    /// will be undefined if the open method didn't set any value.
    ///
    /// write_flags: will contain FUSE_WRITE_CACHE, if this write is from the page cache. If set,
    /// the pid, uid, gid, and fh may not match the value that would have been sent if write cachin
    /// is disabled
    /// flags: these are the file flags, such as O_SYNC. Only supported with ABI >= 7.9
    /// lock_owner: only supported with ABI >= 7.9
    pub fn fuse_write(
        &self,
        ino: u64,
        fh: u64,
        offset: i64,
        data: &[u8],
        write_flags: u32,
        flags: i32,
        lock_owner: Option<u64>,
    ) -> Result<usize> {
        let write_size = self.write_at(ino as u32, offset as usize, data)?;
        Ok(write_size)
    }

    /// Open a directory.
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in fh, and
    /// use this in other all other directory stream operations (readdir, releasedir,
    /// fsyncdir). Filesystem may also implement stateless directory I/O and not store
    /// anything in fh, though that makes it impossible to implement standard conforming
    /// directory stream operations in case the contents of the directory can change
    /// between opendir and releasedir.
    pub fn fuse_opendir(&mut self, ino: u64, flags: i32) -> Result<usize> {
        let inode_ref = self.get_inode_ref(ino as u32);

        // 检查是否为目录
        if !inode_ref.inode.is_dir() {
            return_errno_with_message!(Errno::ENOTDIR, "Not a directory");
        }

        // // 检查权限（例如，只允许读取目录）
        // let file_perm = inode_ref.inode.file_perm();
        // if !file_perm.contains(InodePerm::S_IREAD) {
        //     return_errno_with_message!(Errno::EACCES, "Permission denied");
        // }

        // 成功打开目录，返回文件句柄（这里假设返回 inode 编号作为文件句柄）
        Ok(ino as usize)
    }

    /// Read directory.
    /// Send a buffer filled using buffer.fill(), with size not exceeding the
    /// requested size. Send an empty buffer on end of stream. fh will contain the
    /// value set by the opendir method, or will be undefined if the opendir method
    /// didn't set any value.
    pub fn fuse_readdir(&self, ino: u64, fh: u64, offset: i64) -> Result<Vec<Ext4DirEntry>> {
        let mut entries = self.dir_get_entries(ino as u32);
        entries = entries[offset as usize..].to_vec();
        Ok(entries)
    }

    /// Create and open a file.
    /// If the file does not exist, first create it with the specified mode, and then
    /// open it. Open flags (with the exception of O_NOCTTY) are available in flags.
    /// Filesystem may store an arbitrary file handle (pointer, index, etc) in fh,
    /// and use this in other all other file operations (read, write, flush, release,
    /// fsync). There are also some flags (direct_io, keep_cache) which the
    /// filesystem may set, to change the way the file is opened. See fuse_file_info
    /// structure in <fuse_common.h> for more details. If this method is not
    /// implemented or under Linux kernel versions earlier than 2.6.15, the mknod()
    /// and open() methods will be called instead.
    pub fn fuse_create(
        &mut self,
        parent: u64,
        name: &str,
        mode: u32,
        umask: u32,
        flags: i32,
    ) -> Result<usize> {
        // check file exist
        let mut search_result = Ext4DirSearchResult::new(Ext4DirEntry::default());
        let r = self.dir_find_entry(parent as u32, name, &mut search_result);
        if r.is_ok() {
            let inode_ref = self.get_inode_ref(search_result.dentry.inode);

            // check permission
            let file_perm = inode_ref.inode.file_perm();

            let can_read = file_perm.contains(InodePerm::S_IREAD);
            let can_write = file_perm.contains(InodePerm::S_IWRITE);
            let can_execute = file_perm.contains(InodePerm::S_IEXEC);

            // If trying to open the file in write mode, check for write permissions
            if ((flags & O_WRONLY != 0) || (flags & O_RDWR != 0)) && !can_write {
                return_errno_with_message!(Errno::EACCES, "Permission denied can not write");
            }

            // If trying to open the file in read mode, check for read permissions
            if (flags & O_RDONLY != 0) || (flags & O_RDWR != 0) && !can_read{
                return_errno_with_message!(Errno::EACCES, "Permission denied can not read");
            }

            // If trying to open the file in read mode, check for read permissions
            if (flags & O_EXCL != 0) || (flags & O_RDWR != 0) && !can_execute {
                return_errno_with_message!(Errno::EACCES, "Permission denied can not exec");
            }

            return Ok(EOK);
        } else {
            //create file
            let inode_ref = self.create(parent as u32, name, mode as u16)?;
        }

        Ok(EOK)
    }

    /// Check file access permissions.
    /// This will be called for the access() system call. If the 'default_permissions'
    /// mount option is given, this method is not called. This method is not called
    /// under Linux kernel versions 2.4.x
    /// int access(const char *pathname, int mode);
    /// int faccessat(int dirfd, const char *pathname, int mode, int flags);
    /// 
    /// uid and gid come from request
    pub fn fuse_access(&mut self, ino: u64, uid: u16, gid: u16, mode: u16, mask: i32) -> bool {
        let inode_ref = self.get_inode_ref(ino as u32);

        inode_ref.inode.check_access(uid, gid, mode, mask as u16)
    }

    /// Get file system statistics.
    /// Linux stat syscall defines:
    /// int stat(const char *restrict pathname, struct stat *restrict statbuf);
    /// int fstatat(int dirfd, const char *restrict pathname, struct stat *restrict statbuf, int flags);
    pub fn fuse_statfs(&mut self, ino: u64) -> Result<LinuxStat> {
        let inode_ref = self.get_inode_ref(ino as u32);
        let linux_stat = LinuxStat::from_inode_ref(&inode_ref);
        Ok(linux_stat)
    }

    /// Initialize filesystem.
    /// Called before any other filesystem method.
    /// The kernel module connection can be configured using the KernelConfig object
    pub fn fuse_init(&mut self) -> Result<usize> {
        Ok(EOK)
    }

    /// Clean up filesystem.
    /// Called on filesystem exit.
    pub fn fuse_destroy(&mut self) -> Result<usize> {
        Ok(EOK)
    }

    /// Rename a file.
    fn fuse_rename(&mut self, parent: u64, name: &str, newparent: u64, newname: &str, flags: u32) {
        unimplemented!();
    }

    /// Flush method.
    /// This is called on each close() of the opened file. Since file descriptors can
    /// be duplicated (dup, dup2, fork), for one open call there may be many flush
    /// calls. Filesystems shouldn't assume that flush will always be called after some
    /// writes, or that if will be called at all. fh will contain the value set by the
    /// open method, or will be undefined if the open method didn't set any value.
    /// NOTE: the name of the method is misleading, since (unlike fsync) the filesystem
    /// is not forced to flush pending writes. One reason to flush data, is if the
    /// filesystem wants to return write errors. If the filesystem supports file locking
    /// operations (setlk, getlk) it should remove all locks belonging to 'lock_owner'.
    fn fuse_flush(&mut self, ino: u64, fh: u64, lock_owner: u64) {
        unimplemented!();
    }

    /// Release an open file.
    /// Release is called when there are no more references to an open file: all file
    /// descriptors are closed and all memory mappings are unmapped. For every open
    /// call there will be exactly one release call. The filesystem may reply with an
    /// error, but error values are not returned to close() or munmap() which triggered
    /// the release. fh will contain the value set by the open method, or will be undefined
    /// if the open method didn't set any value. flags will contain the same flags as for
    /// open.
    fn fuse_release(
        &mut self,
        _ino: u64,
        _fh: u64,
        _flags: i32,
        _lock_owner: Option<u64>,
        _flush: bool,
    ) {
        unimplemented!();
    }

    /// Synchronize file contents.
    /// If the datasync parameter is non-zero, then only the user data should be flushed,
    /// not the meta data.
    fn fuse_fsync(&mut self, ino: u64, fh: u64, datasync: bool) {
        unimplemented!();
    }

    /// Read directory.
    /// Send a buffer filled using buffer.fill(), with size not exceeding the
    /// requested size. Send an empty buffer on end of stream. fh will contain the
    /// value set by the opendir method, or will be undefined if the opendir method
    /// didn't set any value.
    fn fuse_readdirplus(&mut self, ino: u64, fh: u64, offset: i64) {
        unimplemented!();
    }

    /// Release an open directory.
    /// For every opendir call there will be exactly one releasedir call. fh will
    /// contain the value set by the opendir method, or will be undefined if the
    /// opendir method didn't set any value.
    fn fuse_releasedir(&mut self, _ino: u64, _fh: u64, _flags: i32) {
        unimplemented!();
    }

    /// Synchronize directory contents.
    /// If the datasync parameter is set, then only the directory contents should
    /// be flushed, not the meta data. fh will contain the value set by the opendir
    /// method, or will be undefined if the opendir method didn't set any value.
    fn fuse_fsyncdir(&mut self, ino: u64, fh: u64, datasync: bool) {
        unimplemented!();
    }

    /// Set an extended attribute.
    fn fuse_setxattr(&mut self, ino: u64, name: &str, _value: &[u8], flags: i32, position: u32) {
        unimplemented!();
    }

    /// Get an extended attribute.
    /// If `size` is 0, the size of the value should be sent with `reply.size()`.
    /// If `size` is not 0, and the value fits, send it with `reply.data()`, or
    /// `reply.error(ERANGE)` if it doesn't.
    fn fuse_getxattr(&mut self, ino: u64, name: &str, size: u32) {
        unimplemented!();
    }

    /// List extended attribute names.
    /// If `size` is 0, the size of the value should be sent with `reply.size()`.
    /// If `size` is not 0, and the value fits, send it with `reply.data()`, or
    /// `reply.error(ERANGE)` if it doesn't.
    fn fuse_listxattr(&mut self, ino: u64, size: u32) {
        unimplemented!();
    }

    /// Remove an extended attribute.
    fn fuse_removexattr(&mut self, ino: u64, name: &str) {
        unimplemented!();
    }

    /// Test for a POSIX file lock.
    fn fuse_getlk(
        &mut self,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
    ) {
        unimplemented!();
    }

    /// Acquire, modify or release a POSIX file lock.
    /// For POSIX threads (NPTL) there's a 1-1 relation between pid and owner, but
    /// otherwise this is not always the case.  For checking lock ownership,
    /// 'fi->owner' must be used. The l_pid field in 'struct flock' should only be
    /// used to fill in this field in getlk(). Note: if the locking methods are not
    /// implemented, the kernel will still allow file locking to work locally.
    /// Hence these are only interesting for network filesystems and similar.
    fn fuse_setlk(
        &mut self,
        ino: u64,
        fh: u64,
        lock_owner: u64,
        start: u64,
        end: u64,
        typ: i32,
        pid: u32,
        sleep: bool,
    ) {
        unimplemented!();
    }

    /// Map block index within file to block index within device.
    /// Note: This makes sense only for block device backed filesystems mounted
    /// with the 'blkdev' option
    fn fuse_bmap(&mut self, ino: u64, blocksize: u32, idx: u64) {
        unimplemented!();
    }

    /// control device
    fn fuse_ioctl(
        &mut self,
        ino: u64,
        fh: u64,
        flags: u32,
        cmd: u32,
        in_data: &[u8],
        out_size: u32,
    ) {
        unimplemented!();
    }

    /// Poll for events
    // #[cfg(feature = "abi-7-11")]
    // fn fuse_poll(
    //     &mut self,
    //     ino: u64,
    //     fh: u64,
    //     kh: u64,
    //     events: u32,
    //     flags: u32,
    // ) {
    // }

    /// Preallocate or deallocate space to a file
    fn fuse_fallocate(&mut self, ino: u64, fh: u64, offset: i64, length: i64, mode: i32) {
        unimplemented!();
    }

    /// Reposition read/write file offset
    fn fuse_lseek(&mut self, ino: u64, fh: u64, offset: i64, whence: i32) {
        unimplemented!();
    }

    /// Copy the specified range from the source inode to the destination inode
    fn fuse_copy_file_range(
        &mut self,
        ino_in: u64,
        fh_in: u64,
        offset_in: i64,
        ino_out: u64,
        fh_out: u64,
        offset_out: i64,
        len: u64,
        flags: u32,
    ) {
        unimplemented!();
    }
}
