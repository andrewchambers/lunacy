//! Unbuffered libc I/O.

use crate::{Errno, fd::BorrowedFd, sys};

/// Calls libc read once and returns the number of bytes read.
///
/// Short reads, EOF (zero), and EINTR are returned directly. The buffer is
/// caller-owned and initialized; bytes beyond the returned count are unchanged
/// on success. No retry or allocation is performed.
pub fn read(fd: BorrowedFd<'_>, buffer: &mut [u8]) -> Result<usize, Errno> {
    // SAFETY: The descriptor stays open and C has exclusive access to the buffer.
    sys::cvt_size(unsafe { sys::lunacy_read(fd.as_raw_fd(), buffer.as_mut_ptr(), buffer.len()) })
}

/// Calls libc write once and returns the number of bytes written.
///
/// Short writes and EINTR are returned to the caller. No retries, buffering,
/// allocation, or changes to signal handling (including SIGPIPE) are performed.
pub fn write(fd: BorrowedFd<'_>, buffer: &[u8]) -> Result<usize, Errno> {
    // SAFETY: The descriptor stays open and the buffer is readable for its length.
    let result = unsafe { sys::lunacy_write(fd.as_raw_fd(), buffer.as_ptr(), buffer.len()) };
    if result == -1 {
        Err(Errno::last())
    } else {
        Ok(result as usize)
    }
}
