//! Unix file descriptor wrappers.

use core::{
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign},
    time::Duration,
};

use crate::{Errno, ErrnoName, Result};

/// A raw Unix file descriptor.
pub type RawFd = crate::ffi::c_int;

/// The standard input file descriptor.
pub const STDIN: RawFd = 0;

/// The standard output file descriptor.
pub const STDOUT: RawFd = 1;

/// The standard error file descriptor.
pub const STDERR: RawFd = 2;

/// A seek position for [`lseek`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeekFrom {
    /// Seek from the beginning of the file.
    Start(i64),
    /// Seek from the current file offset.
    Current(i64),
    /// Seek from the end of the file.
    End(i64),
}

const POLLIN_NAME: crate::ffi::c_int = 1;
const POLLOUT_NAME: crate::ffi::c_int = 2;
const POLLPRI_NAME: crate::ffi::c_int = 3;
const POLLERR_NAME: crate::ffi::c_int = 4;
const POLLHUP_NAME: crate::ffi::c_int = 5;
const POLLNVAL_NAME: crate::ffi::c_int = 6;
const POLLRDHUP_NAME: crate::ffi::c_int = 7;

/// A set of `poll` readiness flags.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct PollEvents(crate::ffi::c_int);

impl PollEvents {
    /// An empty set of poll events.
    pub const EMPTY: Self = Self(0);

    /// Returns libc's `POLLIN` value.
    pub fn input() -> Result<Self> {
        poll_constant(POLLIN_NAME).map(Self)
    }

    /// Returns libc's `POLLOUT` value.
    pub fn output() -> Result<Self> {
        poll_constant(POLLOUT_NAME).map(Self)
    }

    /// Returns libc's `POLLPRI` value.
    pub fn priority() -> Result<Self> {
        poll_constant(POLLPRI_NAME).map(Self)
    }

    /// Returns libc's `POLLERR` value.
    pub fn error() -> Result<Self> {
        poll_constant(POLLERR_NAME).map(Self)
    }

    /// Returns libc's `POLLHUP` value.
    pub fn hangup() -> Result<Self> {
        poll_constant(POLLHUP_NAME).map(Self)
    }

    /// Returns libc's `POLLNVAL` value.
    pub fn invalid() -> Result<Self> {
        poll_constant(POLLNVAL_NAME).map(Self)
    }

    /// Returns libc's `POLLRDHUP` value.
    ///
    /// Some libcs do not expose this flag; in that case this returns an errno.
    pub fn read_hangup() -> Result<Self> {
        poll_constant(POLLRDHUP_NAME).map(Self)
    }

    /// Wraps raw poll event bits.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw poll event bits.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }

    /// Returns whether no flags are set.
    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// Returns whether all flags in `other` are set.
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl BitOr for PollEvents {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for PollEvents {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for PollEvents {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for PollEvents {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// One descriptor entry for [`poll`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct PollFd {
    fd: RawFd,
    events: PollEvents,
    revents: PollEvents,
}

impl PollFd {
    /// Creates a poll entry for `fd`.
    pub fn new(fd: impl FdArg, events: PollEvents) -> Self {
        Self {
            fd: fd.raw_fd(),
            events,
            revents: PollEvents::EMPTY,
        }
    }

    /// Returns the raw descriptor being polled.
    pub const fn raw_fd(self) -> RawFd {
        self.fd
    }

    /// Returns the requested event flags.
    pub const fn events(self) -> PollEvents {
        self.events
    }

    /// Replaces the requested event flags.
    pub fn set_events(&mut self, events: PollEvents) {
        self.events = events;
    }

    /// Returns the event flags reported by the most recent [`poll`] call.
    pub const fn revents(self) -> PollEvents {
        self.revents
    }

    /// Returns whether the most recent [`poll`] call reported any events.
    pub const fn is_ready(self) -> bool {
        !self.revents.is_empty()
    }
}

/// One descriptor entry for [`select`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(C)]
pub struct SelectFd {
    fd: RawFd,
    ready: crate::ffi::c_int,
}

impl SelectFd {
    /// Creates a select entry for `fd`.
    pub fn new(fd: impl FdArg) -> Self {
        Self {
            fd: fd.raw_fd(),
            ready: 0,
        }
    }

    /// Returns the raw descriptor being selected on.
    pub const fn raw_fd(self) -> RawFd {
        self.fd
    }

    /// Returns whether the most recent [`select`] call reported readiness.
    pub const fn is_ready(self) -> bool {
        self.ready != 0
    }
}

/// An owned Unix file descriptor.
///
/// This closes the descriptor with libc `close` when dropped.
#[derive(Debug)]
#[repr(transparent)]
pub struct OwnedFd {
    fd: RawFd,
}

impl OwnedFd {
    /// Creates an owned file descriptor from a raw descriptor.
    ///
    /// # Safety
    ///
    /// `fd` must be open, owned by the caller, and not require cleanup other
    /// than `close`.
    pub unsafe fn from_raw_fd(fd: RawFd) -> Self {
        assert!(fd >= 0, "file descriptor must be non-negative");

        Self { fd }
    }

    /// Borrows this file descriptor.
    pub fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }

    /// Returns the raw file descriptor without transferring ownership.
    pub const fn as_raw_fd(&self) -> RawFd {
        self.fd
    }

    /// Consumes this value and returns the raw file descriptor.
    ///
    /// The caller becomes responsible for closing the descriptor.
    pub fn into_raw_fd(self) -> RawFd {
        let fd = self.fd;
        core::mem::forget(self);
        fd
    }

    /// Closes this file descriptor.
    ///
    /// The descriptor is consumed even if `close` reports an error.
    pub fn close(self) -> Result<()> {
        close(self)
    }
}

impl Drop for OwnedFd {
    fn drop(&mut self) {
        let _ = close_raw(self.fd);
    }
}

/// A borrowed Unix file descriptor.
///
/// The lifetime represents how long the descriptor is guaranteed to remain
/// open by something else.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub struct BorrowedFd<'fd> {
    fd: RawFd,
    _marker: PhantomData<&'fd OwnedFd>,
}

impl<'fd> BorrowedFd<'fd> {
    /// Borrows a raw file descriptor.
    ///
    /// # Safety
    ///
    /// `fd` must remain open for the returned lifetime.
    pub const unsafe fn borrow_raw(fd: RawFd) -> Self {
        assert!(fd >= 0, "file descriptor must be non-negative");

        Self {
            fd,
            _marker: PhantomData,
        }
    }

    /// Returns the raw file descriptor.
    pub const fn as_raw_fd(self) -> RawFd {
        self.fd
    }
}

/// A value that can be passed as a file descriptor argument.
///
/// Bad descriptor numbers are ordinary OS errors, so this trait is safe to
/// implement for raw descriptors as well as borrowed descriptor wrappers.
pub trait FdArg {
    /// Returns the raw file descriptor used for a syscall argument.
    fn raw_fd(self) -> RawFd;
}

impl FdArg for RawFd {
    fn raw_fd(self) -> RawFd {
        self
    }
}

impl FdArg for BorrowedFd<'_> {
    fn raw_fd(self) -> RawFd {
        self.fd
    }
}

impl FdArg for &OwnedFd {
    fn raw_fd(self) -> RawFd {
        self.fd
    }
}

impl FdArg for &mut OwnedFd {
    fn raw_fd(self) -> RawFd {
        self.fd
    }
}

impl FdArg for &BorrowedFd<'_> {
    fn raw_fd(self) -> RawFd {
        self.fd
    }
}

/// Creates a pipe.
///
/// The first returned descriptor is the read end. The second is the write end.
pub fn pipe() -> Result<(OwnedFd, OwnedFd)> {
    let mut fds = [0; 2];
    cvt_int(unsafe { crate::ffi::pipe(fds.as_mut_ptr()) })?;

    let read = unsafe { OwnedFd::from_raw_fd(fds[0]) };
    let write = unsafe { OwnedFd::from_raw_fd(fds[1]) };
    Ok((read, write))
}

/// Duplicates a borrowed file descriptor.
pub fn dup(fd: impl FdArg) -> Result<OwnedFd> {
    cvt_fd(unsafe { crate::ffi::dup(fd.raw_fd()) })
}

/// Duplicates `fd` onto `target`.
///
/// The target descriptor is consumed and replaced by the duplicate on success.
/// If duplication fails, the target descriptor is consumed and closed.
pub fn dup2(fd: impl FdArg, target: OwnedFd) -> Result<OwnedFd> {
    let mut target = ManuallyDrop::new(target);
    let target_raw = target.as_raw_fd();
    let duplicated = unsafe { crate::ffi::dup2(fd.raw_fd(), target_raw) };

    if duplicated < 0 {
        unsafe {
            ManuallyDrop::drop(&mut target);
        }
        Err(Errno::last())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(duplicated) })
    }
}

/// Duplicates `fd` onto a raw target descriptor number.
///
/// On success, the returned [`OwnedFd`] owns `target`.
pub fn dup2_raw(fd: impl FdArg, target: RawFd) -> Result<OwnedFd> {
    cvt_fd(unsafe { crate::ffi::dup2(fd.raw_fd(), target) })
}

/// Reads from a file descriptor into `buf`.
pub fn read(fd: impl FdArg, buf: &mut [u8]) -> Result<usize> {
    read_raw(fd.raw_fd(), buf)
}

/// Writes `buf` to a file descriptor.
pub fn write(fd: impl FdArg, buf: &[u8]) -> Result<usize> {
    write_raw(fd.raw_fd(), buf)
}

/// Writes all bytes in `buf` to a file descriptor.
///
/// `EINTR` is retried. Other errors are returned as [`Errno`].
pub fn write_all(fd: impl FdArg, mut buf: &[u8]) -> Result<()> {
    let fd = fd.raw_fd();
    while !buf.is_empty() {
        match write(fd, buf) {
            Ok(0) => return Err(Errno::eio()),
            Ok(n) => buf = &buf[n..],
            Err(errno) if errno == Errno::eintr() => {}
            Err(errno) => return Err(errno),
        }
    }

    Ok(())
}

/// Closes an owned file descriptor.
///
/// The descriptor is consumed even if `close` reports an error.
pub fn close(fd: OwnedFd) -> Result<()> {
    let fd = fd.into_raw_fd();
    close_raw(fd)
}

/// Synchronizes a file descriptor's in-core state with storage.
pub fn fsync(fd: impl FdArg) -> Result<()> {
    cvt_int(unsafe { crate::ffi::fsync(fd.raw_fd()) }).map(|_| ())
}

/// Repositions a seekable file descriptor.
pub fn lseek(fd: impl FdArg, pos: SeekFrom) -> Result<i64> {
    let fd = fd.raw_fd();
    let mut out = 0_i64;
    let status = match pos {
        SeekFrom::Start(offset) => unsafe { lunacy_lseek_set(fd, offset, &mut out) },
        SeekFrom::Current(offset) => unsafe { lunacy_lseek_current(fd, offset, &mut out) },
        SeekFrom::End(offset) => unsafe { lunacy_lseek_end(fd, offset, &mut out) },
    };

    cvt_int(status).map(|_| out)
}

/// Returns whether close-on-exec is set for `fd`.
pub fn close_on_exec(fd: impl FdArg) -> Result<bool> {
    cvt_bool(unsafe { lunacy_get_cloexec(fd.raw_fd()) })
}

/// Sets or clears close-on-exec for `fd`.
pub fn set_close_on_exec(fd: impl FdArg, enabled: bool) -> Result<()> {
    cvt_int(unsafe { lunacy_set_cloexec(fd.raw_fd(), c_bool(enabled)) }).map(|_| ())
}

/// Returns whether nonblocking mode is set for `fd`.
pub fn nonblocking(fd: impl FdArg) -> Result<bool> {
    cvt_bool(unsafe { lunacy_get_nonblocking(fd.raw_fd()) })
}

/// Sets or clears nonblocking mode for `fd`.
pub fn set_nonblocking(fd: impl FdArg, enabled: bool) -> Result<()> {
    cvt_int(unsafe { lunacy_set_nonblocking(fd.raw_fd(), c_bool(enabled)) }).map(|_| ())
}

/// Waits for readiness on a set of file descriptors using libc `poll`.
///
/// A `None` timeout blocks indefinitely. `Some(Duration::ZERO)` checks
/// readiness without blocking.
pub fn poll(fds: &mut [PollFd], timeout: Option<Duration>) -> Result<usize> {
    let timeout = duration_millis(timeout)?;
    cvt_int(unsafe { lunacy_poll_fds(fds.as_mut_ptr(), fds.len(), timeout) })
        .map(|ready| ready as usize)
}

/// Waits for readiness on descriptor sets using libc `select`.
///
/// A `None` timeout blocks indefinitely. `Some(Duration::ZERO)` checks
/// readiness without blocking. Descriptors must fit in the target libc's
/// `fd_set`.
pub fn select(
    read: &mut [SelectFd],
    write: &mut [SelectFd],
    except: &mut [SelectFd],
    timeout: Option<Duration>,
) -> Result<usize> {
    let timeout = duration_micros(timeout)?;
    cvt_int(unsafe {
        lunacy_select_fds(
            read.as_mut_ptr(),
            read.len(),
            write.as_mut_ptr(),
            write.len(),
            except.as_mut_ptr(),
            except.len(),
            timeout,
        )
    })
    .map(|ready| ready as usize)
}

/// Reads from a raw file descriptor into `buf`.
pub fn read_raw(fd: RawFd, buf: &mut [u8]) -> Result<usize> {
    let read = unsafe { crate::ffi::read(fd, buf.as_mut_ptr().cast(), buf.len()) };
    cvt_ssize(read)
}

/// Writes `buf` to a raw file descriptor.
pub fn write_raw(fd: RawFd, buf: &[u8]) -> Result<usize> {
    let written = unsafe { crate::ffi::write(fd, buf.as_ptr().cast(), buf.len()) };
    cvt_ssize(written)
}

/// Writes all bytes in `buf` to a raw file descriptor.
pub fn write_all_raw(fd: RawFd, mut buf: &[u8]) -> Result<()> {
    while !buf.is_empty() {
        match write_raw(fd, buf) {
            Ok(0) => return Err(Errno::eio()),
            Ok(n) => buf = &buf[n..],
            Err(errno) if errno == Errno::eintr() => {}
            Err(errno) => return Err(errno),
        }
    }

    Ok(())
}

/// Closes a raw file descriptor.
pub fn close_raw(fd: RawFd) -> Result<()> {
    cvt_int(unsafe { crate::ffi::close(fd) }).map(|_| ())
}

fn cvt_fd(value: RawFd) -> Result<OwnedFd> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(value) })
    }
}

fn cvt_int(value: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value)
    }
}

fn cvt_ssize(value: crate::ffi::ssize_t) -> Result<usize> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value as usize)
    }
}

fn cvt_bool(value: crate::ffi::c_int) -> Result<bool> {
    cvt_int(value).map(|value| value != 0)
}

fn poll_constant(name: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    let mut value = 0;
    cvt_int(unsafe { lunacy_poll_constant(name, &mut value) }).map(|_| value)
}

fn duration_millis(timeout: Option<Duration>) -> Result<i64> {
    let Some(timeout) = timeout else {
        return Ok(-1);
    };

    duration_units(timeout, 1_000, 1_000_000)
}

fn duration_micros(timeout: Option<Duration>) -> Result<i64> {
    let Some(timeout) = timeout else {
        return Ok(-1);
    };

    duration_units(timeout, 1_000_000, 1_000)
}

fn duration_units(timeout: Duration, units_per_second: u64, nanos_per_unit: u32) -> Result<i64> {
    let seconds_units = timeout
        .as_secs()
        .checked_mul(units_per_second)
        .ok_or_else(invalid_argument)?;
    let extra_units = if timeout.subsec_nanos() == 0 {
        0
    } else {
        u64::from(timeout.subsec_nanos().div_ceil(nanos_per_unit))
    };
    let units = seconds_units
        .checked_add(extra_units)
        .ok_or_else(invalid_argument)?;

    if units > i64::MAX as u64 {
        Err(invalid_argument())
    } else {
        Ok(units as i64)
    }
}

fn invalid_argument() -> Errno {
    Errno::named(ErrnoName::EINVAL).expect("target libc must define EINVAL")
}

fn c_bool(value: bool) -> crate::ffi::c_int {
    if value { 1 } else { 0 }
}

unsafe extern "C" {
    fn lunacy_lseek_set(fd: RawFd, offset: i64, out: *mut i64) -> crate::ffi::c_int;
    fn lunacy_lseek_current(fd: RawFd, offset: i64, out: *mut i64) -> crate::ffi::c_int;
    fn lunacy_lseek_end(fd: RawFd, offset: i64, out: *mut i64) -> crate::ffi::c_int;
    fn lunacy_get_cloexec(fd: RawFd) -> crate::ffi::c_int;
    fn lunacy_set_cloexec(fd: RawFd, enabled: crate::ffi::c_int) -> crate::ffi::c_int;
    fn lunacy_get_nonblocking(fd: RawFd) -> crate::ffi::c_int;
    fn lunacy_set_nonblocking(fd: RawFd, enabled: crate::ffi::c_int) -> crate::ffi::c_int;
    fn lunacy_poll_constant(
        name: crate::ffi::c_int,
        out: *mut crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_poll_fds(
        fds: *mut PollFd,
        count: crate::ffi::size_t,
        timeout_ms: i64,
    ) -> crate::ffi::c_int;
    fn lunacy_select_fds(
        read_fds: *mut SelectFd,
        read_count: crate::ffi::size_t,
        write_fds: *mut SelectFd,
        write_count: crate::ffi::size_t,
        except_fds: *mut SelectFd,
        except_count: crate::ffi::size_t,
        timeout_us: i64,
    ) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::{
        PollEvents, PollFd, STDERR, STDIN, STDOUT, SeekFrom, SelectFd, close, close_on_exec, dup,
        dup2, lseek, nonblocking, pipe, poll, read, select, set_close_on_exec, set_nonblocking,
        write_all,
    };

    #[test]
    fn exposes_standard_file_descriptors() {
        assert_eq!(STDIN, 0);
        assert_eq!(STDOUT, 1);
        assert_eq!(STDERR, 2);
    }

    #[test]
    fn round_trips_bytes_through_pipe() {
        let (read_fd, write_fd) = pipe().unwrap();

        write_all(&write_fd, b"hello").unwrap();
        close(write_fd).unwrap();

        let mut buf = [0; 8];
        let n = read(&read_fd, &mut buf).unwrap();
        close(read_fd).unwrap();

        assert_eq!(&buf[..n], b"hello");
    }

    #[test]
    fn poll_reports_pipe_readiness() {
        let (read_fd, write_fd) = pipe().unwrap();
        let input = PollEvents::input().unwrap();
        let mut fds = [PollFd::new(&read_fd, input)];

        assert_eq!(poll(&mut fds, Some(Duration::ZERO)).unwrap(), 0);
        assert!(!fds[0].is_ready());

        write_all(&write_fd, b"x").unwrap();

        assert_eq!(poll(&mut fds, Some(Duration::from_millis(100))).unwrap(), 1);
        assert!(fds[0].revents().contains(input));

        close(read_fd).unwrap();
        close(write_fd).unwrap();
    }

    #[test]
    fn select_reports_pipe_readiness() {
        let (read_fd, write_fd) = pipe().unwrap();
        let mut read_fds = [SelectFd::new(&read_fd)];
        let mut write_fds = [SelectFd::new(&write_fd)];
        let mut except_fds = [];

        assert_eq!(
            select(
                &mut read_fds,
                &mut [],
                &mut except_fds,
                Some(Duration::ZERO)
            )
            .unwrap(),
            0
        );
        assert!(!read_fds[0].is_ready());

        assert_eq!(
            select(
                &mut [],
                &mut write_fds,
                &mut except_fds,
                Some(Duration::ZERO)
            )
            .unwrap(),
            1
        );
        assert!(write_fds[0].is_ready());

        write_all(&write_fd, b"x").unwrap();
        assert_eq!(
            select(
                &mut read_fds,
                &mut [],
                &mut except_fds,
                Some(Duration::from_millis(100))
            )
            .unwrap(),
            1
        );
        assert!(read_fds[0].is_ready());

        close(read_fd).unwrap();
        close(write_fd).unwrap();
    }

    #[test]
    fn can_round_trip_owned_fd_to_raw() {
        let (read_fd, write_fd) = pipe().unwrap();
        let read_raw = read_fd.as_raw_fd();
        let write_raw = write_fd.as_raw_fd();

        assert_eq!(write_fd.as_raw_fd(), write_raw);

        let raw_read = read_fd.into_raw_fd();
        assert_eq!(raw_read, read_raw);
        super::close_raw(raw_read).unwrap();
        close(write_fd).unwrap();
    }

    #[test]
    fn duplicates_descriptors_and_sets_flags() {
        let (read_fd, write_fd) = pipe().unwrap();
        let duplicated = dup(&write_fd).unwrap();

        set_close_on_exec(&duplicated, true).unwrap();
        assert!(close_on_exec(&duplicated).unwrap());

        set_nonblocking(&read_fd, true).unwrap();
        assert!(nonblocking(&read_fd).unwrap());
    }

    #[test]
    fn lseek_reports_pipe_error() {
        let (read_fd, write_fd) = pipe().unwrap();

        assert!(lseek(&read_fd, SeekFrom::Start(0)).is_err());
        close(write_fd).unwrap();
    }

    #[test]
    fn dup2_replaces_target_descriptor() {
        let (read_a, write_a) = pipe().unwrap();
        let (read_b, write_b) = pipe().unwrap();

        let replaced = dup2(&write_a, write_b).unwrap();
        write_all(&replaced, b"x").unwrap();
        close(write_a).unwrap();
        close(replaced).unwrap();

        let mut buf = [0; 1];
        let n = read(&read_a, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"x");

        close(read_a).unwrap();
        close(read_b).unwrap();
    }
}
