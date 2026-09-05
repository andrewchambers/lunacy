use lunacy::{
    Errno,
    time::{Clock, Timespec, clock_getres, clock_gettime, nanosleep},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn duration(value: Timespec) -> Duration {
    assert!(value.tv_sec >= 0);
    assert!((0..1_000_000_000).contains(&value.tv_nsec));
    Duration::new(value.tv_sec as u64, value.tv_nsec as u32)
}

#[test]
fn realtime_matches_system_time() {
    let before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let value = duration(clock_gettime(Clock::Realtime).unwrap());
    let after = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    assert!(before <= value && value <= after);
}

#[test]
fn native_clock_ids_and_resolutions() {
    for (clock, native) in [
        (Clock::Realtime, libc::CLOCK_REALTIME),
        (Clock::Monotonic, libc::CLOCK_MONOTONIC),
        (Clock::ProcessCpuTime, libc::CLOCK_PROCESS_CPUTIME_ID),
        (Clock::ThreadCpuTime, libc::CLOCK_THREAD_CPUTIME_ID),
    ] {
        assert_eq!(clock.to_raw(), i64::from(native));
        duration(clock_gettime(clock).unwrap());
        let resolution = clock_getres(clock).unwrap();
        assert!(duration(resolution) > Duration::ZERO);
        assert_eq!(clock_getres(Clock::Raw(clock.to_raw())), Ok(resolution));
    }
}

#[test]
fn sleep_waits_and_leaves_remaining_unchanged_on_success() {
    let request = Timespec {
        tv_sec: 0,
        tv_nsec: 2_000_000,
    };
    let sentinel = Timespec {
        tv_sec: -7,
        tv_nsec: -9,
    };
    let mut remaining = sentinel;
    let before = duration(clock_gettime(Clock::Monotonic).unwrap());
    nanosleep(&request, Some(&mut remaining)).unwrap();
    let after = duration(clock_gettime(Clock::Monotonic).unwrap());
    assert!(after - before >= duration(request));
    assert_eq!(remaining, sentinel);
    nanosleep(&Timespec::default(), None).unwrap();
}

#[test]
fn invalid_sleep_requests_preserve_remaining() {
    let sentinel = Timespec {
        tv_sec: 7,
        tv_nsec: 9,
    };
    for request in [
        Timespec {
            tv_sec: -1,
            tv_nsec: 0,
        },
        Timespec {
            tv_sec: 0,
            tv_nsec: -1,
        },
        Timespec {
            tv_sec: 0,
            tv_nsec: 1_000_000_000,
        },
    ] {
        let mut remaining = sentinel;
        assert_eq!(
            nanosleep(&request, Some(&mut remaining)),
            Err(Errno::EINVAL)
        );
        assert_eq!(remaining, sentinel);
    }
}

#[test]
fn invalid_and_unrepresentable_clock_ids() {
    assert_eq!(clock_gettime(Clock::Raw(-1)), Err(Errno::EINVAL));
    assert_eq!(clock_getres(Clock::Raw(-1)), Err(Errno::EINVAL));
    if size_of::<libc::clockid_t>() < size_of::<i64>() {
        assert_eq!(clock_gettime(Clock::Raw(i64::MAX)), Err(Errno::EOVERFLOW));
        assert_eq!(clock_getres(Clock::Raw(i64::MAX)), Err(Errno::EOVERFLOW));
    }
}

#[test]
fn sleep_rejects_seconds_that_do_not_fit_native_time_t() {
    if size_of::<libc::time_t>() < size_of::<i64>() {
        let request = Timespec {
            tv_sec: i64::MAX,
            tv_nsec: 0,
        };
        assert_eq!(nanosleep(&request, None), Err(Errno::EOVERFLOW));
    }
}
