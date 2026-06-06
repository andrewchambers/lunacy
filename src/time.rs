//! Time and sleep wrappers.
//!
//! Clock IDs are resolved through the C shim so libc-specific values are not
//! baked into Rust constants.

use core::time::Duration;

use crate::{Errno, ErrnoName, Result};

const CLOCK_REALTIME_NAME: crate::ffi::c_int = 1;
const CLOCK_MONOTONIC_NAME: crate::ffi::c_int = 2;
const CLOCK_PROCESS_CPUTIME_ID_NAME: crate::ffi::c_int = 3;
const CLOCK_THREAD_CPUTIME_ID_NAME: crate::ffi::c_int = 4;
const CLOCK_MONOTONIC_RAW_NAME: crate::ffi::c_int = 5;

/// A libc clock identifier.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct ClockId(crate::ffi::c_int);

impl ClockId {
    /// Returns libc's `CLOCK_REALTIME` value.
    pub fn realtime() -> Result<Self> {
        clock_constant(CLOCK_REALTIME_NAME).map(Self)
    }

    /// Returns libc's `CLOCK_MONOTONIC` value.
    pub fn monotonic() -> Result<Self> {
        clock_constant(CLOCK_MONOTONIC_NAME).map(Self)
    }

    /// Returns libc's `CLOCK_PROCESS_CPUTIME_ID` value.
    pub fn process_cputime() -> Result<Self> {
        clock_constant(CLOCK_PROCESS_CPUTIME_ID_NAME).map(Self)
    }

    /// Returns libc's `CLOCK_THREAD_CPUTIME_ID` value.
    pub fn thread_cputime() -> Result<Self> {
        clock_constant(CLOCK_THREAD_CPUTIME_ID_NAME).map(Self)
    }

    /// Returns libc's `CLOCK_MONOTONIC_RAW` value.
    ///
    /// Some libcs do not expose this clock; in that case this returns an errno.
    pub fn monotonic_raw() -> Result<Self> {
        clock_constant(CLOCK_MONOTONIC_RAW_NAME).map(Self)
    }

    /// Wraps a raw clock ID value.
    pub const fn raw(raw: crate::ffi::c_int) -> Self {
        Self(raw)
    }

    /// Returns the raw clock ID value.
    pub const fn as_raw(self) -> crate::ffi::c_int {
        self.0
    }
}

/// A seconds plus nanoseconds timestamp.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Timespec {
    seconds: i64,
    nanoseconds: i32,
}

impl Timespec {
    /// Creates a timestamp from seconds and nanoseconds.
    ///
    /// `nanoseconds` must be in `0..1_000_000_000`.
    pub fn new(seconds: i64, nanoseconds: i32) -> Result<Self> {
        validate_fraction(nanoseconds, 1_000_000_000)?;

        Ok(Self {
            seconds,
            nanoseconds,
        })
    }

    /// Returns the seconds field.
    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    /// Returns the nanoseconds field.
    pub const fn nanoseconds(self) -> i32 {
        self.nanoseconds
    }

    /// Converts this timestamp to a duration when it is non-negative.
    pub fn as_duration(self) -> Option<Duration> {
        if self.seconds < 0 || self.nanoseconds < 0 {
            None
        } else {
            Some(Duration::new(self.seconds as u64, self.nanoseconds as u32))
        }
    }
}

/// A seconds plus microseconds timestamp.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(C)]
pub struct Timeval {
    seconds: i64,
    microseconds: i32,
}

impl Timeval {
    /// Creates a timestamp from seconds and microseconds.
    ///
    /// `microseconds` must be in `0..1_000_000`.
    pub fn new(seconds: i64, microseconds: i32) -> Result<Self> {
        validate_fraction(microseconds, 1_000_000)?;

        Ok(Self {
            seconds,
            microseconds,
        })
    }

    /// Returns the seconds field.
    pub const fn seconds(self) -> i64 {
        self.seconds
    }

    /// Returns the microseconds field.
    pub const fn microseconds(self) -> i32 {
        self.microseconds
    }

    /// Converts this timestamp to a duration when it is non-negative.
    pub fn as_duration(self) -> Option<Duration> {
        if self.seconds < 0 || self.microseconds < 0 {
            None
        } else {
            Some(Duration::new(
                self.seconds as u64,
                self.microseconds as u32 * 1_000,
            ))
        }
    }
}

/// The result of one `nanosleep` call.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepStatus {
    /// The requested duration elapsed.
    Complete,
    /// The call was interrupted and returned the remaining duration.
    Interrupted(Duration),
}

/// Returns the current time for `clock`.
pub fn clock_gettime(clock: ClockId) -> Result<Timespec> {
    let mut out = Timespec::default();
    cvt_int(unsafe { lunacy_clock_gettime(clock.as_raw(), &mut out) }).map(|_| out)
}

/// Returns the current `CLOCK_REALTIME` value.
pub fn realtime() -> Result<Timespec> {
    clock_gettime(ClockId::realtime()?)
}

/// Returns the current `CLOCK_MONOTONIC` value.
pub fn monotonic() -> Result<Timespec> {
    clock_gettime(ClockId::monotonic()?)
}

/// Returns the current wall-clock time from libc `gettimeofday`.
pub fn gettimeofday() -> Result<Timeval> {
    let mut out = Timeval::default();
    cvt_int(unsafe { lunacy_gettimeofday(&mut out) }).map(|_| out)
}

/// Returns the current Unix time in seconds using libc `time`.
pub fn time() -> Result<i64> {
    let mut out = 0;
    cvt_int(unsafe { lunacy_time_seconds(&mut out) }).map(|_| out)
}

/// Calls libc `nanosleep` once.
///
/// If the sleep is interrupted by a signal, this returns
/// [`SleepStatus::Interrupted`] with libc's remaining duration.
pub fn nanosleep(duration: Duration) -> Result<SleepStatus> {
    let (seconds, nanoseconds) = duration_parts(duration)?;
    let mut remaining_seconds = 0;
    let mut remaining_nanoseconds = 0;
    let status = unsafe {
        lunacy_nanosleep(
            seconds,
            nanoseconds,
            &mut remaining_seconds,
            &mut remaining_nanoseconds,
        )
    };

    if status == 0 {
        return Ok(SleepStatus::Complete);
    }

    let errno = Errno::last();
    if errno == Errno::eintr() {
        let remaining = duration_from_parts(remaining_seconds, remaining_nanoseconds)?;
        Ok(SleepStatus::Interrupted(remaining))
    } else {
        Err(errno)
    }
}

/// Sleeps until `duration` has elapsed.
///
/// Interrupted sleeps are retried with the remaining duration.
pub fn sleep(duration: Duration) -> Result<()> {
    let mut remaining = duration;
    while remaining != Duration::ZERO {
        match nanosleep(remaining)? {
            SleepStatus::Complete => return Ok(()),
            SleepStatus::Interrupted(duration) if duration == Duration::ZERO => return Ok(()),
            SleepStatus::Interrupted(duration) => remaining = duration,
        }
    }

    Ok(())
}

fn clock_constant(name: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    let mut value = 0;
    cvt_int(unsafe { lunacy_clock_constant(name, &mut value) }).map(|_| value)
}

fn cvt_int(value: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value)
    }
}

fn duration_parts(duration: Duration) -> Result<(i64, i32)> {
    if duration.as_secs() > i64::MAX as u64 {
        return Err(invalid_argument());
    }

    Ok((duration.as_secs() as i64, duration.subsec_nanos() as i32))
}

fn duration_from_parts(seconds: i64, nanoseconds: i32) -> Result<Duration> {
    if seconds < 0 {
        return Err(invalid_argument());
    }

    validate_fraction(nanoseconds, 1_000_000_000)?;
    Ok(Duration::new(seconds as u64, nanoseconds as u32))
}

fn validate_fraction(value: i32, max: i32) -> Result<()> {
    if value < 0 || value >= max {
        Err(invalid_argument())
    } else {
        Ok(())
    }
}

fn invalid_argument() -> Errno {
    Errno::named(ErrnoName::EINVAL).expect("target libc must define EINVAL")
}

unsafe extern "C" {
    fn lunacy_clock_constant(
        name: crate::ffi::c_int,
        out: *mut crate::ffi::c_int,
    ) -> crate::ffi::c_int;
    fn lunacy_clock_gettime(clock_id: crate::ffi::c_int, out: *mut Timespec) -> crate::ffi::c_int;
    fn lunacy_gettimeofday(out: *mut Timeval) -> crate::ffi::c_int;
    fn lunacy_time_seconds(out: *mut i64) -> crate::ffi::c_int;
    fn lunacy_nanosleep(
        seconds: i64,
        nanoseconds: i32,
        remaining_seconds: *mut i64,
        remaining_nanoseconds: *mut i32,
    ) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use core::time::Duration;

    use super::{
        ClockId, SleepStatus, Timespec, Timeval, clock_gettime, gettimeofday, monotonic, nanosleep,
        realtime, sleep, time,
    };

    #[test]
    fn creates_valid_time_values() {
        assert_eq!(Timespec::new(1, 2).unwrap().nanoseconds(), 2);
        assert_eq!(Timeval::new(1, 2).unwrap().microseconds(), 2);
        assert!(Timespec::new(1, 1_000_000_000).is_err());
        assert!(Timeval::new(1, 1_000_000).is_err());
    }

    #[test]
    fn reads_realtime_and_monotonic_clocks() {
        let real = realtime().unwrap();
        let mono = monotonic().unwrap();

        assert!(real.seconds() > 0);
        assert!(mono.nanoseconds() >= 0);
        assert!(clock_gettime(ClockId::monotonic().unwrap()).is_ok());
    }

    #[test]
    fn reads_gettimeofday() {
        let tv = gettimeofday().unwrap();

        assert!(tv.seconds() > 0);
        assert!((0..1_000_000).contains(&tv.microseconds()));
    }

    #[test]
    fn reads_unix_time_seconds() {
        assert!(time().unwrap() > 0);
    }

    #[test]
    fn nanosleep_zero_completes() {
        assert_eq!(nanosleep(Duration::ZERO).unwrap(), SleepStatus::Complete);
    }

    #[test]
    fn sleeps_for_small_duration() {
        sleep(Duration::from_millis(1)).unwrap();
    }
}
