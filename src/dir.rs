//! Owned libc directory streams with entries borrowed until the next operation.
//!
//! Uses readdir's native storage, without copying names or assuming NAME_MAX.
//! Requires libc to allow concurrent readdir calls on distinct streams.

use crate::{Errno, sys};
use core::{
    ffi::{CStr, c_char, c_int, c_void},
    mem::ManuallyDrop,
    ptr::NonNull,
};

unsafe extern "C" {
    fn lunacy_opendir(path: *const c_char) -> *mut c_void;
    fn lunacy_readdir(directory: *mut c_void, name: *mut *const c_char, ino: *mut u64) -> c_int;
    fn lunacy_closedir(directory: *mut c_void) -> c_int;
}

/// An exclusively owned DIR*, closed once on drop. Drop ignores close errors;
/// [`closedir`] reports them. Moving a stream between threads is allowed;
/// reading requires exclusive access.
pub struct Dir {
    raw: NonNull<c_void>,
}

// SAFETY: The pointer remains stable and a moved owner transfers exclusive
// access to the stream. No entry borrow may survive moving the owner.
unsafe impl Send for Dir {}

impl Drop for Dir {
    fn drop(&mut self) {
        // SAFETY: This is the stream's only owner and no entry borrows remain.
        unsafe { lunacy_closedir(self.raw.as_ptr()) };
    }
}

/// A native directory entry's portable fields. No filtering or sorting is done.
/// Names may be non-UTF-8; `.` and `..` are returned when supplied by libc.
///
/// The name cannot outlive or overlap another read of the stream:
/// ```compile_fail
/// use lunacy::{Dir, readdir};
/// fn invalid(dir: &mut Dir) {
///     let first = readdir(dir).unwrap().unwrap();
///     let second = readdir(dir);
///     let name = first.d_name;
/// }
/// ```
/// ```compile_fail
/// use lunacy::{Dir, readdir, closedir};
/// fn invalid(mut dir: Dir) {
///     let entry = readdir(&mut dir).unwrap().unwrap();
///     closedir(dir);
///     let name = entry.d_name;
/// }
/// ```
#[derive(Debug)]
pub struct DirEntry<'dir> {
    /// Native inode number.
    pub d_ino: u64,
    /// Name borrowed from the stream's current entry.
    pub d_name: &'dir CStr,
}

/// Opens a directory stream. libc owns its internal allocation and descriptor.
pub fn opendir(path: &CStr) -> Result<Dir, Errno> {
    // SAFETY: path is terminated. A successful call gives exclusive ownership.
    let raw = unsafe { lunacy_opendir(path.as_ptr()) };
    NonNull::new(raw)
        .map(|raw| Dir { raw })
        .ok_or_else(Errno::last)
}

/// Calls readdir once. Ok(None) means end of stream; errors remain distinct.
/// The returned name borrows the stream until the entry's last use. Use
/// `entry.d_name.to_owned()` when a name must survive another stream operation.
pub fn readdir(directory: &mut Dir) -> Result<Option<DirEntry<'_>>, Errno> {
    let mut name = core::ptr::null();
    let mut ino = 0;
    // SAFETY: This exclusive borrow prevents concurrent reads and close. Both
    // outputs are writable; the shim distinguishes EOF from error using errno.
    let rc = sys::cvt(unsafe { lunacy_readdir(directory.raw.as_ptr(), &mut name, &mut ino) })?;
    if rc == 0 {
        return Ok(None);
    }
    // SAFETY: Successful readdir supplies a terminated name whose storage is
    // valid until the next read/close; the exclusive borrow prevents both.
    Ok(Some(DirEntry {
        d_ino: ino,
        d_name: unsafe { CStr::from_ptr(name) },
    }))
}

/// Consumes the stream and calls closedir once. On error the stream cannot be
/// reused, and the call is not retried.
pub fn closedir(directory: Dir) -> Result<(), Errno> {
    let directory = ManuallyDrop::new(directory);
    // SAFETY: Ownership is consumed and automatic close has been suppressed.
    sys::cvt(unsafe { lunacy_closedir(directory.raw.as_ptr()) }).map(|_| ())
}
