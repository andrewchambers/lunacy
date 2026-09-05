//! libc clocks and relative sleeping, without allocation or automatic retries.

use core::ptr;

use crate::{Errno, sys};

/// Seconds and nanoseconds, converted to/from native timespec fields by C.
///
/// This is not a native ABI timespec. Nanoseconds must be in 0..1_000_000_000.
/// Clock timestamps and relative intervals use the same representation, as in C;
/// the caller is responsible for choosing the appropriate clock and operation.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Timespec {
    /// Whole seconds. Sleep requests require a nonnegative value.
    pub tv_sec: i64,
    /// Nanosecond fraction, in 0..1_000_000_000.
    pub tv_nsec: i64,
}

/// A clock identifier, resolved using the target's C headers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Clock {
    /// CLOCK_REALTIME: Unix wall-clock time, subject to system-clock changes.
    Realtime,
    /// CLOCK_MONOTONIC: elapsed time from an unspecified origin, without
    /// discontinuous wall-clock jumps. Suspend accounting is platform-specific.
    Monotonic,
    /// CLOCK_PROCESS_CPUTIME_ID: CPU time consumed by the current process.
    ProcessCpuTime,
    /// CLOCK_THREAD_CPUTIME_ID: CPU time consumed by the calling thread.
    ThreadCpuTime,
    /// A native clock ID supplied by the caller. Values that do not fit the
    /// target's clockid_t return EOVERFLOW from clock_gettime/clock_getres.
    Raw(i64),
}

unsafe extern "C" {
    fn lunacy_clock_realtime() -> i64;
    fn lunacy_clock_monotonic() -> i64;
    fn lunacy_clock_process_cputime() -> i64;
    fn lunacy_clock_thread_cputime() -> i64;
}

impl Clock {
    /// Returns the native clock ID without assuming its numeric assignment.
    pub fn to_raw(self) -> i64 {
        // SAFETY: The C accessors only read native clock constants.
        unsafe {
            match self {
                Self::Realtime => lunacy_clock_realtime(),
                Self::Monotonic => lunacy_clock_monotonic(),
                Self::ProcessCpuTime => lunacy_clock_process_cputime(),
                Self::ThreadCpuTime => lunacy_clock_thread_cputime(),
                Self::Raw(raw) => raw,
            }
        }
    }
}

/// Calls clock_gettime once, returning the selected clock's timestamp.
/// Unsupported clocks return the native error; no fallback clock is substituted.
pub fn clock_gettime(clock: Clock) -> Result<Timespec, Errno> {
    let raw = clock.to_raw();
    let mut value = Timespec::default();
    // SAFETY: Both fields are writable and C converts the native layout/types.
    sys::cvt(unsafe { sys::lunacy_clock_gettime(raw, &mut value.tv_sec, &mut value.tv_nsec) })?;
    Ok(value)
}

/// Calls clock_getres once, returning the selected clock's resolution.
/// Nanosecond units do not imply nanosecond precision or sleep scheduling.
pub fn clock_getres(clock: Clock) -> Result<Timespec, Errno> {
    let raw = clock.to_raw();
    let mut value = Timespec::default();
    // SAFETY: Both fields are writable and C converts the native layout/types.
    sys::cvt(unsafe { sys::lunacy_clock_getres(raw, &mut value.tv_sec, &mut value.tv_nsec) })?;
    Ok(value)
}

/// Calls nanosleep once to suspend the calling thread for a relative interval.
///
/// On EINTR, an optional remaining duration is filled by libc. The caller decides
/// whether to resume sleeping; this function never retries. Remaining storage is
/// left unchanged on success or other errors. Actual sleep may exceed the request
/// because of clock granularity and scheduling.
///
/// Negative seconds or invalid nanoseconds return EINVAL. Seconds that cannot
/// fit the target's time_t return EOVERFLOW instead of being truncated.
pub fn nanosleep(request: &Timespec, remaining: Option<&mut Timespec>) -> Result<(), Errno> {
    let (seconds, nanoseconds) = match remaining {
        Some(value) => (&mut value.tv_sec as *mut _, &mut value.tv_nsec as *mut _),
        None => (ptr::null_mut(), ptr::null_mut()),
    };
    // SAFETY: Output fields are either both null or exclusive writable references.
    // C validates and converts the request before passing it to libc.
    sys::cvt(unsafe {
        sys::lunacy_nanosleep(request.tv_sec, request.tv_nsec, seconds, nanoseconds)
    })
    .map(|_| ())
}
