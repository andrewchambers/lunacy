//! File and path operations. Paths are C strings; calls do not retry.

use crate::{
    Errno,
    fd::{BorrowedFd, OwnedFd},
    sys,
    time::Timespec,
};
use core::{
    ffi::{CStr, c_char, c_int},
    ops::{BitOr, BitOrAssign},
};

macro_rules! native_mask {
    ($(#[$doc:meta])* $name:ident($raw:ty) { $($(#[$item_doc:meta])* $getter:ident => $symbol:ident),+ $(,)? }) => {
        $(#[$doc])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
        pub struct $name($raw);
        unsafe extern "C" { $(fn $symbol() -> $raw;)+ }
        impl $name {
            /// Wraps native bits, including platform-specific values.
            pub const fn from_raw(raw: $raw) -> Self { Self(raw) }
            /// Returns the native bits.
            pub const fn to_raw(self) -> $raw { self.0 }
            /// Returns zero bits.
            pub const fn empty() -> Self { Self(0) }
            $(
                $(#[$item_doc])*
                pub fn $getter() -> Self {
                    // SAFETY: The accessor reads a native constant.
                    Self(unsafe { $symbol() })
                }
            )+
        }
        impl BitOr for $name {
            type Output = Self;
            fn bitor(self, rhs: Self) -> Self { Self(self.0 | rhs.0) }
        }
        impl BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) { self.0 |= rhs.0; }
        }
    };
}

native_mask! {
    /// Native open flags. Choose one access mode, then OR additional flags.
    /// O_RDONLY may be zero; access modes are not independent flag bits.
    OpenFlags(c_int) {
        /// O_RDONLY: read only.
        rdonly => lunacy_o_rdonly,
        /// O_WRONLY: write only.
        wronly => lunacy_o_wronly,
        /// O_RDWR: read and write.
        rdwr => lunacy_o_rdwr,
        /// O_CREAT: create a missing file using the supplied mode and umask.
        creat => lunacy_o_creat,
        /// O_EXCL: with O_CREAT, fail if the path exists.
        excl => lunacy_o_excl,
        /// O_TRUNC: truncate an existing writable regular file.
        trunc => lunacy_o_trunc,
        /// O_APPEND: append each write.
        append => lunacy_o_append,
        /// O_CLOEXEC: close the descriptor on exec.
        cloexec => lunacy_o_cloexec,
        /// O_DIRECTORY: require a directory.
        directory => lunacy_o_directory,
        /// O_NOFOLLOW: do not follow the final symbolic link.
        nofollow => lunacy_o_nofollow,
        /// O_NONBLOCK: request nonblocking operation.
        nonblock => lunacy_o_nonblock,
        /// O_SYNC: request synchronized writes.
        sync => lunacy_o_sync,
    }
}

native_mask! {
    /// Native mode bits, used for creation permissions and returned by stat.
    Mode(u32) {
        /// S_IRUSR: owner read permission.
        irusr => lunacy_s_irusr,
        /// S_IWUSR: owner write permission.
        iwusr => lunacy_s_iwusr,
        /// S_IXUSR: owner execute/search permission.
        ixusr => lunacy_s_ixusr,
        /// S_IRGRP: group read permission.
        irgrp => lunacy_s_irgrp,
        /// S_IWGRP: group write permission.
        iwgrp => lunacy_s_iwgrp,
        /// S_IXGRP: group execute/search permission.
        ixgrp => lunacy_s_ixgrp,
        /// S_IROTH: other read permission.
        iroth => lunacy_s_iroth,
        /// S_IWOTH: other write permission.
        iwoth => lunacy_s_iwoth,
        /// S_IXOTH: other execute/search permission.
        ixoth => lunacy_s_ixoth,
        /// S_ISUID: set-user-ID bit.
        isuid => lunacy_s_isuid,
        /// S_ISGID: set-group-ID bit.
        isgid => lunacy_s_isgid,
        /// S_ISVTX: sticky bit.
        isvtx => lunacy_s_isvtx,
    }
}

/// File type determined by the native S_IS* macros.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    /// Regular file.
    Regular,
    /// Directory.
    Directory,
    /// Symbolic link.
    Symlink,
    /// Character device.
    Character,
    /// Block device.
    Block,
    /// FIFO/pipe.
    Fifo,
    /// Socket.
    Socket,
    /// No recognized native file type.
    Unknown,
}

impl Mode {
    /// Applies the native S_IS* tests to these mode bits.
    pub fn file_type(self) -> FileType {
        // SAFETY: C inspects mode bits without accessing memory.
        match unsafe { lunacy_file_type(self.0) } {
            1 => FileType::Regular,
            2 => FileType::Directory,
            3 => FileType::Symlink,
            4 => FileType::Character,
            5 => FileType::Block,
            6 => FileType::Fifo,
            7 => FileType::Socket,
            _ => FileType::Unknown,
        }
    }
}

/// Common stat fields, explicitly converted from native struct stat by C.
/// This is the shim's layout, not the OS ABI. Platform extensions are omitted.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stat {
    /// Device containing the file.
    pub st_dev: u64,
    /// Inode number.
    pub st_ino: u64,
    /// Hard link count.
    pub st_nlink: u64,
    /// Owner user ID.
    pub st_uid: u64,
    /// Owner group ID.
    pub st_gid: u64,
    /// Device ID for a special file.
    pub st_rdev: u64,
    /// Native file type and permission bits.
    pub st_mode: Mode,
    /// Size in bytes (meaning depends on file type).
    pub st_size: i64,
    /// Last access timestamp.
    pub st_atim: Timespec,
    /// Last data modification timestamp.
    pub st_mtim: Timespec,
    /// Last metadata change timestamp; not creation time.
    pub st_ctim: Timespec,
}

unsafe extern "C" {
    fn lunacy_open(path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn lunacy_openat(cwd: c_int, fd: c_int, path: *const c_char, flags: c_int, mode: u32) -> c_int;
    fn lunacy_stat(path: *const c_char, out: *mut Stat) -> c_int;
    fn lunacy_lstat(path: *const c_char, out: *mut Stat) -> c_int;
    fn lunacy_fstat(fd: c_int, out: *mut Stat) -> c_int;
    fn lunacy_file_type(mode: u32) -> c_int;
    fn lunacy_seek_set() -> c_int;
    fn lunacy_seek_cur() -> c_int;
    fn lunacy_seek_end() -> c_int;
    fn lunacy_lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn lunacy_ftruncate(fd: c_int, length: i64) -> c_int;
    fn lunacy_unlink(path: *const c_char) -> c_int;
    fn lunacy_rename(old: *const c_char, new: *const c_char) -> c_int;
    fn lunacy_mkdir(path: *const c_char, mode: u32) -> c_int;
    fn lunacy_rmdir(path: *const c_char) -> c_int;
    fn lunacy_getcwd(buffer: *mut u8, capacity: usize) -> c_int;
    fn lunacy_chdir(path: *const c_char) -> c_int;
    fn lunacy_isatty(fd: c_int) -> c_int;
    fn lunacy_mkstemp(template: *mut u8) -> c_int;
}

fn owned_result(raw: c_int) -> Result<OwnedFd, Errno> {
    let raw = sys::cvt(raw)?;
    // SAFETY: Used only with calls that return a fresh descriptor on success.
    Ok(unsafe { OwnedFd::from_raw_fd(raw) })
}

/// Opens a file once. Mode supplies creation permissions, subject to umask;
/// it is ignored by libc when the flags do not request creation.
/// No flags (including close-on-exec) are added implicitly.
pub fn open(path: &CStr, flags: OpenFlags, mode: Mode) -> Result<OwnedFd, Errno> {
    // SAFETY: path is terminated, and C supplies the mode vararg with its native type.
    owned_result(unsafe { lunacy_open(path.as_ptr(), flags.0, mode.0) })
}

/// Opens relative to a borrowed directory, or AT_FDCWD when `directory` is None.
/// An absolute path ignores the directory, as in libc.
pub fn openat(
    directory: Option<BorrowedFd<'_>>,
    path: &CStr,
    flags: OpenFlags,
    mode: Mode,
) -> Result<OwnedFd, Errno> {
    // SAFETY: The optional descriptor stays open; path is a valid C string.
    owned_result(unsafe {
        lunacy_openat(
            directory.is_none() as c_int,
            directory.map_or(-1, BorrowedFd::as_raw_fd),
            path.as_ptr(),
            flags.0,
            mode.0,
        )
    })
}

/// Reads metadata, following the final symbolic link.
pub fn stat(path: &CStr) -> Result<Stat, Errno> {
    let mut value = Stat::default();
    // SAFETY: The shim receives a terminated path and a writable matching layout.
    sys::cvt(unsafe { lunacy_stat(path.as_ptr(), &mut value) })?;
    Ok(value)
}

/// Reads metadata for the final symbolic link itself, when present.
pub fn lstat(path: &CStr) -> Result<Stat, Errno> {
    let mut value = Stat::default();
    // SAFETY: The shim receives a terminated path and a writable matching layout.
    sys::cvt(unsafe { lunacy_lstat(path.as_ptr(), &mut value) })?;
    Ok(value)
}

/// Reads metadata from an open descriptor.
pub fn fstat(fd: BorrowedFd<'_>) -> Result<Stat, Errno> {
    let mut value = Stat::default();
    // SAFETY: The borrow keeps fd open; the output matches the shim layout.
    sys::cvt(unsafe { lunacy_fstat(fd.as_raw_fd(), &mut value) })?;
    Ok(value)
}

/// Native seek origins, resolved through the target headers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Whence {
    /// SEEK_SET: offset from the beginning.
    Set,
    /// SEEK_CUR: offset from the current position.
    Cur,
    /// SEEK_END: offset from the end.
    End,
    /// Another native whence value.
    Raw(c_int),
}
impl Whence {
    /// Returns the native whence value.
    pub fn to_raw(self) -> c_int {
        // SAFETY: These accessors read native constants.
        unsafe {
            match self {
                Self::Set => lunacy_seek_set(),
                Self::Cur => lunacy_seek_cur(),
                Self::End => lunacy_seek_end(),
                Self::Raw(raw) => raw,
            }
        }
    }
}

/// Moves the shared file offset and returns its new position. Values that do
/// not fit native off_t return EOVERFLOW. Does not retry EINTR.
pub fn lseek(fd: BorrowedFd<'_>, offset: i64, whence: Whence) -> Result<i64, Errno> {
    let whence = whence.to_raw();
    // SAFETY: fd remains open and C checks the native offset conversion.
    let result = unsafe { lunacy_lseek(fd.as_raw_fd(), offset, whence) };
    if result == -1 {
        Err(Errno::last())
    } else {
        Ok(result)
    }
}

/// Sets a file's length without changing its offset. Native errors are returned;
/// lengths that do not fit off_t return EOVERFLOW.
pub fn ftruncate(fd: BorrowedFd<'_>, length: i64) -> Result<(), Errno> {
    // SAFETY: fd remains open and C checks the native length conversion.
    sys::cvt(unsafe { lunacy_ftruncate(fd.as_raw_fd(), length) }).map(|_| ())
}

/// Removes a directory entry using unlink.
pub fn unlink(path: &CStr) -> Result<(), Errno> {
    // SAFETY: path is a valid C string.
    sys::cvt(unsafe { lunacy_unlink(path.as_ptr()) }).map(|_| ())
}

/// Renames a path, preserving libc's replacement semantics.
pub fn rename(old: &CStr, new: &CStr) -> Result<(), Errno> {
    // SAFETY: Both paths are valid C strings.
    sys::cvt(unsafe { lunacy_rename(old.as_ptr(), new.as_ptr()) }).map(|_| ())
}

/// Creates one directory with the supplied permissions, subject to umask.
pub fn mkdir(path: &CStr, mode: Mode) -> Result<(), Errno> {
    // SAFETY: path is a valid C string; C converts mode to its native type.
    sys::cvt(unsafe { lunacy_mkdir(path.as_ptr(), mode.0) }).map(|_| ())
}

/// Removes one empty directory.
pub fn rmdir(path: &CStr) -> Result<(), Errno> {
    // SAFETY: path is a valid C string.
    sys::cvt(unsafe { lunacy_rmdir(path.as_ptr()) }).map(|_| ())
}

/// Writes the absolute working directory into the caller's buffer and borrows
/// its C string. An empty/insufficient buffer returns ERANGE, without resizing.
pub fn getcwd(buffer: &mut [u8]) -> Result<&CStr, Errno> {
    // SAFETY: The buffer is writable for its length; C rejects zero capacity.
    sys::cvt(unsafe { lunacy_getcwd(buffer.as_mut_ptr(), buffer.len()) })?;
    CStr::from_bytes_until_nul(buffer).map_err(|_| Errno::EIO)
}

/// Changes the process-wide working directory. Other threads' relative path
/// operations observe this change; callers must coordinate when necessary.
pub fn chdir(path: &CStr) -> Result<(), Errno> {
    // SAFETY: path is a valid C string. Changing cwd does not invalidate Rust memory.
    sys::cvt(unsafe { lunacy_chdir(path.as_ptr()) }).map(|_| ())
}

/// Tests for a terminal: ENOTTY becomes Ok(false), other errors are preserved.
pub fn isatty(fd: BorrowedFd<'_>) -> Result<bool, Errno> {
    // SAFETY: The descriptor is borrowed for the duration of the call.
    sys::cvt(unsafe { lunacy_isatty(fd.as_raw_fd()) }).map(|rc| rc != 0)
}

/// Creates and opens a unique file, replacing the final six X bytes in place.
/// `template` must be exactly one NUL-terminated path ending in XXXXXX before
/// the NUL. Invalid buffers return EINVAL. libc chooses permissions and flags;
/// dropping the descriptor closes it but does not unlink the generated path.
pub fn mkstemp(template: &mut [u8]) -> Result<OwnedFd, Errno> {
    let path = CStr::from_bytes_with_nul(template).map_err(|_| Errno::EINVAL)?;
    if !path.to_bytes().ends_with(b"XXXXXX") {
        return Err(Errno::EINVAL);
    }
    // SAFETY: template is a writable terminated path with the required suffix.
    owned_result(unsafe { lunacy_mkstemp(template.as_mut_ptr()) })
}
