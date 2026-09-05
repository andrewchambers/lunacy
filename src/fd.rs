//! File descriptor ownership without buffering or implicit duplication.

use core::{ffi::c_int, marker::PhantomData, mem::ManuallyDrop};

use crate::{Errno, sys};

/// A C file descriptor number, carrying no ownership or validity guarantee.
pub type RawFd = c_int;

/// A descriptor guaranteed to remain open for the borrow's lifetime.
#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct BorrowedFd<'fd> {
    raw: RawFd,
    _owner: PhantomData<&'fd OwnedFd>,
}

impl<'fd> BorrowedFd<'fd> {
    /// Borrows a descriptor managed outside lunacy.
    ///
    /// # Safety
    /// `raw` must be a valid open descriptor and remain open for all of `'fd`.
    pub const unsafe fn borrow_raw(raw: RawFd) -> Self {
        Self {
            raw,
            _owner: PhantomData,
        }
    }

    /// Returns the number without transferring ownership.
    pub const fn as_raw_fd(self) -> RawFd {
        self.raw
    }
}

/// An exclusively owned descriptor, closed once on drop.
///
/// Drop ignores close errors. Use [`close`] to observe them. Neither path retries
/// close: on systems that leave the descriptor open after EINTR, it may leak.
///
/// A borrow prevents closing or transferring its owner while still in use:
/// ```compile_fail
/// use lunacy::fd::{OwnedFd, close};
/// fn invalid(owner: OwnedFd) {
///     let borrowed = owner.as_fd();
///     close(owner);
///     lunacy::io::write(borrowed, b"still open?");
/// }
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct OwnedFd {
    raw: RawFd,
}

impl OwnedFd {
    /// Takes ownership of an externally obtained descriptor.
    ///
    /// # Safety
    /// `raw` must be a valid open descriptor. The caller must transfer exclusive
    /// ownership: no other owner may close it, and existing borrows must end.
    pub const unsafe fn from_raw_fd(raw: RawFd) -> Self {
        Self { raw }
    }

    /// Borrows this descriptor without duplicating it.
    pub fn as_fd(&self) -> BorrowedFd<'_> {
        // SAFETY: This owner cannot be dropped or consumed during the borrow.
        unsafe { BorrowedFd::borrow_raw(self.raw) }
    }

    /// Returns the number without transferring ownership.
    pub const fn as_raw_fd(&self) -> RawFd {
        self.raw
    }

    /// Transfers ownership to the caller, suppressing automatic close.
    #[must_use = "the caller must arrange to close the returned descriptor"]
    pub fn into_raw_fd(self) -> RawFd {
        ManuallyDrop::new(self).raw
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        // SAFETY: The descriptor is exclusively owned; this is its last use.
        unsafe { sys::lunacy_close(self.raw) };
    }
}

/// Calls libc pipe once, returning `(read_end, write_end)` as separate owners.
///
/// Preserves pipe's default flags; neither nonblocking nor close-on-exec is
/// added. EOF is observed once every descriptor for the write end is closed.
pub fn pipe() -> Result<(OwnedFd, OwnedFd), Errno> {
    let mut fds = [-1; 2];
    // SAFETY: fds has room for the two descriptors returned by pipe.
    sys::cvt(unsafe { sys::lunacy_pipe(fds.as_mut_ptr()) })?;
    // SAFETY: Successful pipe returns two fresh, distinct, exclusively owned fds.
    Ok(unsafe { (OwnedFd::from_raw_fd(fds[0]), OwnedFd::from_raw_fd(fds[1])) })
}

/// Calls libc dup once, returning a new owner for the duplicated descriptor.
///
/// Preserves dup semantics, including sharing the file offset and clearing the
/// new descriptor's close-on-exec flag. Does not retry interrupted calls.
pub fn dup(fd: BorrowedFd<'_>) -> Result<OwnedFd, Errno> {
    // SAFETY: The borrow guarantees a live descriptor for this call.
    let raw = unsafe { sys::lunacy_dup(fd.raw) };
    if raw == -1 {
        Err(Errno::last())
    } else {
        // SAFETY: Successful dup returns a fresh descriptor owned by the caller.
        Ok(unsafe { OwnedFd::from_raw_fd(raw) })
    }
}

/// Calls libc dup2 once, replacing the target's open file description.
///
/// The target retains its descriptor number and owner. Its old descriptor is
/// closed atomically as part of dup2; it is not closed separately beforehand.
/// The mutable borrow excludes existing safe borrows of the target. On failure,
/// the caller retains both owners. No interruption retry is performed.
///
/// The duplicate shares the source's offset/status flags, and its close-on-exec
/// flag is cleared. See [`dup2_raw`] for targeting an unowned descriptor number.
///
/// ```compile_fail
/// use lunacy::fd::{OwnedFd, dup2};
/// fn invalid(source: OwnedFd, mut target: OwnedFd) {
///     let borrowed = target.as_fd();
///     dup2(source.as_fd(), &mut target);
///     lunacy::io::write(borrowed, b"old target");
/// }
/// ```
pub fn dup2(source: BorrowedFd<'_>, target: &mut OwnedFd) -> Result<(), Errno> {
    // SAFETY: source remains open, and the exclusive borrow of target permits
    // replacing its underlying file while retaining the same descriptor number.
    sys::cvt(unsafe { sys::lunacy_dup2(source.raw, target.raw) }).map(|_| ())
}

/// Calls libc dup2 once with a raw target number, returning that number.
///
/// On success the caller must arrange ownership of the target; no new Rust owner
/// is constructed here. If source and target are the same number, libc validates
/// it and returns it unchanged; no additional owner should be created.
///
/// # Safety
/// The caller must have exclusive authority to replace `target`, ensuring that
/// closing its previous resource does not invalidate any live owner's or borrower's
/// invariants. Coordinate with other threads that allocate, use, or close that
/// number. When source equals target, its existing owner remains responsible.
pub unsafe fn dup2_raw(source: BorrowedFd<'_>, target: RawFd) -> Result<RawFd, Errno> {
    // SAFETY: The source borrow and caller's exclusive control cover this call.
    sys::cvt(unsafe { sys::lunacy_dup2(source.raw, target) })
}

/// Consumes the owner and calls libc close once, returning any error.
///
/// Ownership is relinquished even on failure. No retry is attempted because
/// some systems release and may reuse the descriptor before reporting an error.
pub fn close(fd: OwnedFd) -> Result<(), Errno> {
    let raw = fd.into_raw_fd();
    // SAFETY: into_raw_fd transferred exclusive ownership to this function.
    if unsafe { sys::lunacy_close(raw) } == -1 {
        Err(Errno::last())
    } else {
        Ok(())
    }
}
