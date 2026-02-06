// Copyright © 2025-2026, __robot@PLT
// SPDX-License-Identifier: MIT

use core::{error::Error, fmt::Display};

use alloc::{alloc::AllocError, collections::TryReserveError};

macro_rules! errno_defs {
    (
        $($desc: literal ; $name: ident = $value: literal ,)+
    ) => {
        /// Errno definitions as used by the antimatter operating system.
        #[repr(i32)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Errno {
            $(
                #[doc = $desc]
                $name = $value,
            )+
        }

        impl Errno {
            pub const fn desc(&self) -> &'static str {
                match self {
                    $(
                        Self::$name => $desc,
                    )+
                }
            }

            pub const fn name(&self) -> &'static str {
                match self {
                    $(
                        Self::$name => stringify!($name),
                    )+
                }
            }
        }
    };
}

errno_defs!(
    "Operation not permitted";
    EPERM   = 1,
    "No such file or directory";
    ENOENT  = 2,
    "No such process";
    ESRCH   = 3,
    "Interrupted system call";
    EINTR   = 4,
    "I/O error";
    EIO     = 5,
    "No such device or address";
    ENXIO   = 6,
    "Argument list too long";
    E2BIG   = 7,
    "Exec format error";
    ENOEXEC = 8,
    "Bad file number";
    EBADF   = 9,
    "No child processes";
    ECHILD  = 10,
    "Try again";
    EAGAIN  = 11,
    "Out of memory";
    ENOMEM  = 12,
    "Permission denied";
    EACCES  = 13,
    "Bad address";
    EFAULT  = 14,
    "Block device required";
    ENOTBLK = 15,
    "Device or resource busy";
    EBUSY   = 16,
    "File exists";
    EEXIST  = 17,
    "Cross-device link";
    EXDEV   = 18,
    "No such device";
    ENODEV  = 19,
    "Not a directory";
    ENOTDIR = 20,
    "Is a directory";
    EISDIR  = 21,
    "Invalid argument";
    EINVAL  = 22,
    "File table overflow";
    ENFILE  = 23,
    "Too many open files";
    EMFILE  = 24,
    "Not a typewriter";
    ENOTTY  = 25,
    "Text file busy";
    ETXTBSY = 26,
    "File too large";
    EFBIG   = 27,
    "No space left on device";
    ENOSPC  = 28,
    "Illegal seek";
    ESPIPE  = 29,
    "Read-only file system";
    EROFS   = 30,
    "Too many links";
    EMLINK  = 31,
    "Broken pipe";
    EPIPE   = 32,
    "Math argument out of domain of func";
    EDOM    = 33,
    "Math result not representable";
    ERANGE  = 34,

    "Resource deadlock would occur";
    EDEADLK      = 35,
    "File name too long";
    ENAMETOOLONG = 36,
    "No locks available";
    ENOLCK       = 37,

    "Invalid system call number";
    ENOSYS = 38,

    "Directory not empty";
    ENOTEMPTY   = 39,
    "Too many symbolic links encountered";
    ELOOP       = 40,
    "Not supported";
    ENOTSUP     = 41,
    "No message of desired type";
    ENOMSG      = 42,
    "Identifier removed";
    EIDRM       = 43,
    "Channel number out of range";
    ECHRNG      = 44,
    "Level 2 not synchronized";
    EL2NSYNC    = 45,
    "Level 3 halted";
    EL3HLT      = 46,
    "Level 3 reset";
    EL3RST      = 47,
    "Link number out of range";
    ELNRNG      = 48,
    "Protocol driver not attached";
    EUNATCH     = 49,
    "No CSI structure available";
    ENOCSI      = 50,
    "Level 2 halted";
    EL2HLT      = 51,
    "Invalid exchange";
    EBADE       = 52,
    "Invalid request descriptor";
    EBADR       = 53,
    "Exchange full";
    EXFULL      = 54,
    "No anode";
    ENOANO      = 55,
    "Invalid request code";
    EBADRQC     = 56,
    "Invalid slot";
    EBADSLT     = 57,

    "Bad font file format";
    EBFONT          = 59,
    "Device not a stream";
    ENOSTR          = 60,
    "No data available";
    ENODATA         = 61,
    "Timer expired";
    ETIME           = 62,
    "Out of streams resources";
    ENOSR           = 63,
    "Machine is not on the network";
    ENONET          = 64,
    "Package not installed";
    ENOPKG          = 65,
    "Object is remote";
    EREMOTE         = 66,
    "Link has been severed";
    ENOLINK         = 67,
    "Advertise error";
    EADV            = 68,
    "Srmount error";
    ESRMNT          = 69,
    "Communication error on send";
    ECOMM           = 70,
    "Protocol error";
    EPROTO          = 71,
    "Multihop attempted";
    EMULTIHOP       = 72,
    "RFS specific error";
    EDOTDOT         = 73,
    "Not a data message";
    EBADMSG         = 74,
    "Value too large for defined data type";
    EOVERFLOW       = 75,
    "Name not unique on network";
    ENOTUNIQ        = 76,
    "File descriptor in bad state";
    EBADFD          = 77,
    "Remote address changed";
    EREMCHG         = 78,
    "Can not access a needed shared library";
    ELIBACC         = 79,
    "Accessing a corrupted shared library";
    ELIBBAD         = 80,
    ".lib section in a.out corrupted";
    ELIBSCN         = 81,
    "Attempting to link in too many shared libraries";
    ELIBMAX         = 82,
    "Cannot exec a shared library directly";
    ELIBEXEC        = 83,
    "Illegal byte sequence";
    EILSEQ          = 84,
    "Interrupted system call should be restarted";
    ERESTART        = 85,
    "Streams pipe error";
    ESTRPIPE        = 86,
    "Too many users";
    EUSERS          = 87,
    "Socket operation on non-socket";
    ENOTSOCK        = 88,
    "Destination address required";
    EDESTADDRREQ    = 89,
    "Message too long";
    EMSGSIZE        = 90,
    "Protocol wrong type for socket";
    EPROTOTYPE      = 91,
    "Protocol not available";
    ENOPROTOOPT     = 92,
    "Protocol not supported";
    EPROTONOSUPPORT = 93,
    "Socket type not supported";
    ESOCKTNOSUPPORT = 94,
    "Operation not supported on transport endpoint";
    EOPNOTSUPP      = 95,
    "Protocol family not supported";
    EPFNOSUPPORT    = 96,
    "Address family not supported by protocol";
    EAFNOSUPPORT    = 97,
    "Address already in use";
    EADDRINUSE      = 98,
    "Cannot assign requested address";
    EADDRNOTAVAIL   = 99,
    "Network is down";
    ENETDOWN        = 100,
    "Network is unreachable";
    ENETUNREACH     = 101,
    "Network dropped connection because of reset";
    ENETRESET       = 102,
    "Software caused connection abort";
    ECONNABORTED    = 103,
    "Connection reset by peer";
    ECONNRESET      = 104,
    "No buffer space available";
    ENOBUFS         = 105,
    "Transport endpoint is already connected";
    EISCONN         = 106,
    "Transport endpoint is not connected";
    ENOTCONN        = 107,
    "Cannot send after transport endpoint shutdown";
    ESHUTDOWN       = 108,
    "Too many references: cannot splice";
    ETOOMANYREFS    = 109,
    "Connection (or other operation) timed out";
    ETIMEDOUT       = 110,
    "Connection refused";
    ECONNREFUSED    = 111,
    "Host is down";
    EHOSTDOWN       = 112,
    "No route to host";
    EHOSTUNREACH    = 113,
    "Operation already in progress";
    EALREADY        = 114,
    "Operation now in progress";
    EINPROGRESS     = 115,
    "Stale file handle";
    ESTALE          = 116,
    "Structure needs cleaning";
    EUCLEAN         = 117,
    "Not a XENIX named type file";
    ENOTNAM         = 118,
    "No XENIX semaphores available";
    ENAVAIL         = 119,
    "Is a named type file";
    EISNAM          = 120,
    "Remote I/O error";
    EREMOTEIO       = 121,
    "Quota exceeded";
    EDQUOT          = 122,

    "No medium found";
    ENOMEDIUM    = 123,
    "Wrong medium type";
    EMEDIUMTYPE  = 124,
    "Operation Canceled";
    ECANCELED    = 125,
    "Required key not available";
    ENOKEY       = 126,
    "Key has expired";
    EKEYEXPIRED  = 127,
    "Key has been revoked";
    EKEYREVOKED  = 128,
    "Key was rejected by service";
    EKEYREJECTED = 129,

    "Owner died";
    EOWNERDEAD      = 130,
    "State not recoverable";
    ENOTRECOVERABLE = 131,
    "Operation not possible due to RF-kill";
    ERFKILL         = 132,
    "Memory page has hardware error";
    EHWPOISON       = 133,

    "Assertion failed";
    EASSERT = 134,
);

impl Errno {
    /// Create an `EResult` from some integer.
    pub fn check_bool(errno: i32) -> EResult<bool> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute(-errno) })
        } else {
            Ok(errno != 0)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_i32(errno: i32) -> EResult<i32> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute(-errno) })
        } else {
            Ok(errno as i32)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_i64(errno: i64) -> EResult<i64> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute((-errno) as u32) })
        } else {
            Ok(errno as i64)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_u32(errno: i32) -> EResult<u32> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute(-errno) })
        } else {
            Ok(errno as u32)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_u64(errno: i64) -> EResult<u64> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute((-errno) as u32) })
        } else {
            Ok(errno as u64)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_isize(errno: isize) -> EResult<isize> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute((-errno) as u32) })
        } else {
            Ok(errno as isize)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check_usize(errno: isize) -> EResult<usize> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute((-errno) as u32) })
        } else {
            Ok(errno as usize)
        }
    }
    /// Create an `EResult` from some integer.
    pub fn check(errno: i32) -> EResult<()> {
        if errno < 0 {
            Err(unsafe { core::mem::transmute(-errno) })
        } else {
            Ok(())
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_i32(res: EResult<i32>) -> i32 {
        match res {
            Ok(x) => x as i32,
            Err(x) => -(x as u32 as i32),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_i64(res: EResult<i64>) -> i64 {
        match res {
            Ok(x) => x as i64,
            Err(x) => -(x as u64 as i64),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_u32(res: EResult<u32>) -> i32 {
        match res {
            Ok(x) => x as i32,
            Err(x) => -(x as u32 as i32),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_u64(res: EResult<u64>) -> i64 {
        match res {
            Ok(x) => x as i64,
            Err(x) => -(x as u64 as i64),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_bool(res: EResult<bool>) -> i32 {
        match res {
            Ok(x) => x as i32,
            Err(x) => -(x as u32 as i32),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract(res: EResult<()>) -> i32 {
        match res {
            Ok(()) => 0,
            Err(x) => -(x as u32 as i32),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_isize(res: EResult<isize>) -> isize {
        match res {
            Ok(x) => x as isize,
            Err(x) => -(x as u32 as isize),
        }
    }
    /// Convert an `EResult` into an integer.
    pub fn extract_usize(res: EResult<usize>) -> isize {
        match res {
            Ok(x) => x as isize,
            Err(x) => -(x as u32 as isize),
        }
    }
}

impl Display for Errno {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} ({})", self.name(), self.desc())
    }
}

impl Error for Errno {}

impl From<AllocError> for Errno {
    fn from(_: AllocError) -> Self {
        Errno::ENOMEM
    }
}

impl From<TryReserveError> for Errno {
    fn from(_: TryReserveError) -> Self {
        Errno::ENOMEM
    }
}

pub type EResult<T> = Result<T, Errno>;
