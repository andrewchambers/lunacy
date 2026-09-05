//! libc select and native FD_* operations, with borrowed descriptor lifetimes.

use crate::{
    Errno,
    fd::{BorrowedFd, OwnedFd, RawFd},
    sys,
};
use core::{ffi::c_int, marker::PhantomData, ptr};

/// A native fd_set. Inserted descriptors must remain open while the set is used.
///
/// Storage stays on the stack; C checks its capacity against the native type.
/// Bounds are checked before invoking FD_SET/FD_CLR to avoid C macro UB.
/// Removing a descriptor does not shorten the set's Rust borrow lifetime.
///
/// ```compile_fail
/// use lunacy::{OwnedFd, FdSet};
/// fn invalid(owner: OwnedFd) {
///     let mut set = FdSet::new();
///     set.insert(owner.as_fd()).unwrap();
///     drop(owner);
///     set.clear();
/// }
/// ```
#[derive(Clone)]
pub struct FdSet<'fd> {
    storage: sys::Storage,
    _owner: PhantomData<&'fd OwnedFd>,
}

impl<'fd> FdSet<'fd> {
    /// Constructs a set cleared with FD_ZERO.
    pub fn new() -> Self {
        let mut set = Self {
            storage: sys::Storage([0; 128]),
            _owner: PhantomData,
        };
        set.clear();
        set
    }

    /// Returns the target's FD_SETSIZE, the exclusive descriptor-number limit.
    pub fn capacity() -> c_int {
        // SAFETY: Reads a C constant.
        unsafe { sys::lunacy_fd_setsize() }
    }

    /// Performs FD_ZERO, clearing all descriptors.
    pub fn clear(&mut self) {
        // SAFETY: C initializes the storage, whose size is checked at build time.
        unsafe { sys::lunacy_fd_zero(&mut self.storage) }
    }

    /// Performs FD_SET. Descriptor numbers outside 0..FD_SETSIZE return EINVAL.
    pub fn insert(&mut self, fd: BorrowedFd<'fd>) -> Result<(), Errno> {
        // SAFETY: The borrowed descriptor remains live; C checks the bit index.
        sys::cvt(unsafe { sys::lunacy_fd_set(&mut self.storage, fd.as_raw_fd()) }).map(|_| ())
    }

    /// Performs FD_CLR. Invalid indices return EINVAL, even for absent entries.
    pub fn remove(&mut self, fd: RawFd) -> Result<(), Errno> {
        // SAFETY: C checks the index before accessing the initialized set.
        sys::cvt(unsafe { sys::lunacy_fd_clr(&mut self.storage, fd) }).map(|_| ())
    }

    /// Performs FD_ISSET. Out-of-range descriptor numbers return false.
    pub fn contains(&self, fd: RawFd) -> bool {
        // SAFETY: C checks the index before accessing the initialized set.
        unsafe { sys::lunacy_fd_isset(&self.storage, fd) != 0 }
    }
}

impl Default for FdSet<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// select's timeout, converted to the native timeval by C with range checks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TimeVal {
    /// Nonnegative seconds.
    pub tv_sec: i64,
    /// Microseconds, in 0..1_000_000.
    pub tv_usec: i64,
}

/// Calls select once, replacing supplied sets with the ready descriptors.
///
/// `nfds` is the highest descriptor to check plus one, in 0..=FD_SETSIZE. `None`
/// skips a set. A null timeout waits indefinitely; zero polls immediately.
/// The result counts set bits across all three sets, not distinct descriptors.
/// Native changes to timeout are copied back on success. Reset it before each
/// subsequent call because libc behavior differs between systems. On error,
/// the supplied sets and timeout are left unchanged; EINTR is not retried.
pub fn select(
    nfds: c_int,
    readfds: Option<&mut FdSet<'_>>,
    writefds: Option<&mut FdSet<'_>>,
    exceptfds: Option<&mut FdSet<'_>>,
    timeout: Option<&mut TimeVal>,
) -> Result<usize, Errno> {
    fn storage(set: Option<&mut FdSet<'_>>) -> *mut sys::Storage {
        set.map_or(ptr::null_mut(), |set| &mut set.storage)
    }
    let (seconds, microseconds) = match timeout {
        Some(tv) => (&mut tv.tv_sec as *mut _, &mut tv.tv_usec as *mut _),
        None => (ptr::null_mut(), ptr::null_mut()),
    };
    // SAFETY: Optional buffers are null or live, exclusive references. C validates
    // nfds/timeval before touching native FD_* macros or invoking select.
    sys::cvt(unsafe {
        sys::lunacy_select(
            nfds,
            storage(readfds),
            storage(writefds),
            storage(exceptfds),
            seconds,
            microseconds,
        )
    })
    .map(|n| n as usize)
}
