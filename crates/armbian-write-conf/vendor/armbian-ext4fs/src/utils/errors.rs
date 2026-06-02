

/// Ext4Error number.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Errno {
    EPERM = 1,         /* Operation not permitted */
    ENOENT = 2,        /* No such file or directory */
    EINTR = 4,         /* Interrupted system call */
    EIO = 5,           /* I/O error */
    ENXIO = 6,         /* No such device or address */
    E2BIG = 7,         /* Argument list too long */
    EBADF = 9,         /* Bad file number */
    EAGAIN = 11,       /* Try again */
    ENOMEM = 12,       /* Out of memory */
    EACCES = 13,       /* Permission denied */
    EFAULT = 14,       /* Bad address */
    ENOTBLK = 15,      /* Block device required */
    EBUSY = 16,        /* Device or resource busy */
    EEXIST = 17,       /* File exists */
    EXDEV = 18,        /* Cross-device link */
    ENODEV = 19,       /* No such device */
    ENOTDIR = 20,      /* Not a directory */
    EISDIR = 21,       /* Is a directory */
    EINVAL = 22,       /* Invalid argument */
    ENFILE = 23,       /* File table overflow */
    EMFILE = 24,       /* Too many open files */
    ENOTTY = 25,       /* Not a typewriter */
    ETXTBSY = 26,      /* Text file busy */
    EFBIG = 27,        /* File too large */
    ENOSPC = 28,       /* No space left on device */
    ESPIPE = 29,       /* Illegal seek */
    EROFS = 30,        /* Read-only file system */
    EMLINK = 31,       /* Too many links */
    EPIPE = 32,        /* Broken pipe */
    ENAMETOOLONG = 36, /* File name too long */
    ENOTSUP   = 95,   /* Not supported */
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(unused)]
pub struct Ext4Error {
    errno: Errno,
    msg: Option<&'static str>,
}

impl Ext4Error {
    pub const fn new(errno: Errno) -> Self {
        Ext4Error { errno, msg: None }
    }

    pub const fn with_message(errno: Errno, msg: &'static str) -> Self {
        Ext4Error {
            errno,
            msg: Some(msg),
        }
    }

    pub const fn error(&self) -> Errno {
        self.errno
    }
}

impl From<Errno> for Ext4Error {
    fn from(errno: Errno) -> Self {
        Ext4Error::new(errno)
    }
}

impl From<core::str::Utf8Error> for Ext4Error {
    fn from(_: core::str::Utf8Error) -> Self {
        Ext4Error::with_message(Errno::EINVAL, "Invalid utf-8 string")
    }
}

impl From<alloc::string::FromUtf8Error> for Ext4Error {
    fn from(_: alloc::string::FromUtf8Error) -> Self {
        Ext4Error::with_message(Errno::EINVAL, "Invalid utf-8 string")
    }
}

#[macro_export]
macro_rules! return_errno {
    ($errno: expr) => {
        return Err(Ext4Error::new($errno))
    };
}

#[macro_export]
macro_rules! return_errno_with_message {
    ($errno: expr, $message: expr) => {
        return Err(Ext4Error::with_message($errno, $message))
    };
}
