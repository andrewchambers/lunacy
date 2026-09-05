//! libc poll with borrowed descriptors and caller-owned event storage.

use crate::{
    Errno,
    fd::{BorrowedFd, OwnedFd, RawFd},
    sys,
};
use core::{
    ffi::{c_int, c_short},
    marker::PhantomData,
    ops::{BitOr, BitOrAssign},
};

/// A native poll event mask. Values come from the target's C headers.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PollEvents(c_short);

macro_rules! events {
    ($($(#[$doc:meta])* $name:ident => $symbol:ident),+ $(,)?) => {
        unsafe extern "C" { $(fn $symbol() -> c_short;)+ }
        impl PollEvents {
            $(
                $(#[$doc])*
                pub fn $name() -> Self {
                    // SAFETY: The shim reads a constant defined by the C headers.
                    Self(unsafe { $symbol() })
                }
            )+
        }
    };
}

events! {
    /// POLLIN: data (or EOF) can be read.
    pollin => lunacy_pollin,
    /// POLLOUT: writing is possible.
    pollout => lunacy_pollout,
    /// POLLPRI: priority data is available.
    pollpri => lunacy_pollpri,
    /// POLLERR: an error occurred, returned even if not requested.
    pollerr => lunacy_pollerr,
    /// POLLHUP: the peer hung up, returned even if not requested.
    pollhup => lunacy_pollhup,
    /// POLLNVAL: descriptor invalid, returned even if not requested.
    pollnval => lunacy_pollnval,
}

impl PollEvents {
    /// An empty event mask.
    pub const fn empty() -> Self {
        Self(0)
    }
    /// Wraps native event bits, including platform-specific flags.
    pub const fn from_raw(bits: c_short) -> Self {
        Self(bits)
    }
    /// Returns the native event bits.
    pub const fn to_raw(self) -> c_short {
        self.0
    }
    /// Whether all bits in `events` are set.
    pub const fn contains(self, events: Self) -> bool {
        self.0 & events.0 == events.0
    }
}

impl BitOr for PollEvents {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PollEvents {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// One native pollfd entry, retaining the descriptor's borrow.
///
/// C checks the pollfd layout against this repr(C) field sequence. `None` in
/// [`Self::new`] represents an ignored entry (native fd -1).
///
/// ```compile_fail
/// use lunacy::{fd::OwnedFd, poll::{PollFd, PollEvents, poll}};
/// fn invalid(owner: OwnedFd) {
///     let mut fds = [PollFd::new(Some(owner.as_fd()), PollEvents::pollin())];
///     drop(owner);
///     poll(&mut fds, 0);
/// }
/// ```
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct PollFd<'fd> {
    fd: c_int,
    events: c_short,
    revents: c_short,
    _owner: PhantomData<&'fd OwnedFd>,
}

impl<'fd> PollFd<'fd> {
    /// Creates an entry with no returned events yet.
    pub fn new(fd: Option<BorrowedFd<'fd>>, events: PollEvents) -> Self {
        Self {
            fd: fd.map_or(-1, BorrowedFd::as_raw_fd),
            events: events.0,
            revents: 0,
            _owner: PhantomData,
        }
    }
    /// Returns the native descriptor number, or -1 for an ignored entry.
    pub const fn as_raw_fd(&self) -> RawFd {
        self.fd
    }
    /// Returns the requested native event mask.
    pub const fn events(&self) -> PollEvents {
        PollEvents(self.events)
    }
    /// Changes the requested event mask. Returned events change on the next poll.
    pub fn set_events(&mut self, events: PollEvents) {
        self.events = events.0;
    }
    /// Returns revents from the last successful poll call.
    pub const fn revents(&self) -> PollEvents {
        PollEvents(self.revents)
    }
}

/// Calls poll once and updates each entry's revents.
///
/// Timeout is milliseconds: zero returns immediately and any negative value
/// waits indefinitely. The return value counts entries with nonzero revents,
/// not individual event bits. EINTR is returned without retrying. On failure,
/// revents should be treated as unspecified, as with libc.
pub fn poll(fds: &mut [PollFd<'_>], timeout: c_int) -> Result<usize, Errno> {
    // SAFETY: repr(C) entries match the checked native pollfd layout; the slice
    // is exclusively borrowed, and descriptor lifetimes cover this call.
    sys::cvt(unsafe { sys::lunacy_poll(fds.as_mut_ptr().cast(), fds.len(), timeout) })
        .map(|n| n as usize)
}
