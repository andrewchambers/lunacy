//! Symbolic errors translated using the system C headers.

use core::ffi::c_int;

macro_rules! errno_codes {
    ($($(#[$doc:meta])* $name:ident => $symbol:ident),+ $(,)?) => {
        /// A libc error, independent of the platform's numeric assignments.
        ///
        /// The named variants are an initial subset. All other codes survive
        /// conversion unchanged as `Unknown`. Discriminants are not errno values.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum Errno {
            $($(#[$doc])* $name,)+
            /// An error number without a named variant (including zero).
            Unknown(c_int),
        }

        unsafe extern "C" {
            $(fn $symbol() -> c_int;)+
        }

        impl Errno {
            /// Converts a native errno value into a symbolic error.
            pub fn from_raw(raw: c_int) -> Self {
                $(
                    // SAFETY: Each shim only reads a constant from the C headers.
                    if raw == unsafe { $symbol() } {
                        return Self::$name;
                    }
                )+
                Self::Unknown(raw)
            }

            /// Returns the native errno value on this system.
            pub fn to_raw(self) -> c_int {
                match self {
                    // SAFETY: These shims take no arguments and read C constants.
                    $(Self::$name => unsafe { $symbol() },)+
                    Self::Unknown(raw) => raw,
                }
            }
        }
    };
}

errno_codes! {
    /// Argument and environment data exceeds the exec limit.
    E2BIG => lunacy_e2big,
    /// No matching child process.
    ECHILD => lunacy_echild,
    /// Executable format not recognized.
    ENOEXEC => lunacy_enoexec,
    /// Executable file is busy.
    ETXTBSY => lunacy_etxtbsy,
    /// Permission denied.
    EACCES => lunacy_eacces,
    /// Resource temporarily unavailable (also EWOULDBLOCK where aliased).
    EAGAIN => lunacy_eagain,
    /// Bad file descriptor.
    EBADF => lunacy_ebadf,
    /// File exists.
    EEXIST => lunacy_eexist,
    /// File too large.
    EFBIG => lunacy_efbig,
    /// Interrupted operation.
    EINTR => lunacy_eintr,
    /// Invalid argument.
    EINVAL => lunacy_einval,
    /// Input/output error.
    EIO => lunacy_eio,
    /// Process file descriptor limit reached.
    EMFILE => lunacy_emfile,
    /// System file descriptor limit reached.
    ENFILE => lunacy_enfile,
    /// No such file or directory.
    ENOENT => lunacy_enoent,
    /// Not enough memory.
    ENOMEM => lunacy_enomem,
    /// No space left on device.
    ENOSPC => lunacy_enospc,
    /// Broken pipe.
    EPIPE => lunacy_epipe,
    /// A nonblocking operation is in progress.
    EINPROGRESS => lunacy_einprogress,
    /// An operation is already in progress.
    EALREADY => lunacy_ealready,
    /// Address already in use.
    EADDRINUSE => lunacy_eaddrinuse,
    /// Address unavailable on this system.
    EADDRNOTAVAIL => lunacy_eaddrnotavail,
    /// Address family not supported.
    EAFNOSUPPORT => lunacy_eafnosupport,
    /// Connection refused.
    ECONNREFUSED => lunacy_econnrefused,
    /// Connection reset.
    ECONNRESET => lunacy_econnreset,
    /// Socket is not connected.
    ENOTCONN => lunacy_enotconn,
    /// Descriptor is not a socket.
    ENOTSOCK => lunacy_enotsock,
    /// Protocol not supported.
    EPROTONOSUPPORT => lunacy_eprotonosupport,
    /// Socket operation not supported.
    EOPNOTSUPP => lunacy_eopnotsupp,
    /// Operation timed out.
    ETIMEDOUT => lunacy_etimedout,
    /// Message too large.
    EMSGSIZE => lunacy_emsgsize,
    /// A value does not fit the native representation.
    EOVERFLOW => lunacy_eoverflow,
    /// Resource is busy (including an already locked mutex).
    EBUSY => lunacy_ebusy,
    /// The operation would deadlock.
    EDEADLK => lunacy_edeadlk,
    /// Operation not permitted.
    EPERM => lunacy_eperm,
    /// Buffer or result is out of range.
    ERANGE => lunacy_erange,
    /// A path component is not a directory.
    ENOTDIR => lunacy_enotdir,
    /// Operation is inappropriate for a directory.
    EISDIR => lunacy_eisdir,
    /// Directory is not empty.
    ENOTEMPTY => lunacy_enotempty,
    /// Descriptor is not a terminal.
    ENOTTY => lunacy_enotty,
    /// Descriptor does not support seeking.
    ESPIPE => lunacy_espipe,
    /// Too many symbolic links encountered.
    ELOOP => lunacy_eloop,
    /// Path or filename is too long.
    ENAMETOOLONG => lunacy_enametoolong,
    /// Read-only filesystem.
    EROFS => lunacy_erofs,
    /// Operation crosses filesystem boundaries.
    EXDEV => lunacy_exdev,
}

unsafe extern "C" {
    fn lunacy_strerror_r(error: c_int, buffer: *mut u8, capacity: usize) -> c_int;
}

/// Copies native error text into the caller's buffer, returning a borrowed
/// C string. Uses the POSIX strerror_r interface, including on glibc.
/// Insufficient storage returns ERANGE; unknown codes may return EINVAL.
/// No allocation, UTF-8 conversion, or automatic resizing is performed.
/// The text follows libc's locale; foreign locale mutation must obey libc's
/// synchronization requirements.
pub fn strerror_r(error: Errno, buffer: &mut [u8]) -> Result<&core::ffi::CStr, Errno> {
    let raw = error.to_raw();
    // SAFETY: The writable buffer has the stated size; the shim rejects zero
    // capacity and returns error numbers directly instead of setting errno.
    let rc = unsafe { lunacy_strerror_r(raw, buffer.as_mut_ptr(), buffer.len()) };
    if rc != 0 {
        return Err(Errno::from_raw(rc));
    }
    core::ffi::CStr::from_bytes_until_nul(buffer).map_err(|_| Errno::EIO)
}

impl Errno {
    /// Captures the calling thread's errno, then translates it.
    ///
    /// Only meaningful after a call reports failure using errno. Successful
    /// calls need not clear errno, and subsequent libc calls can overwrite it.
    pub fn last() -> Self {
        // SAFETY: The C shim accesses errno using the system's own definition.
        let raw = unsafe { crate::sys::lunacy_errno() };
        Self::from_raw(raw)
    }
}
