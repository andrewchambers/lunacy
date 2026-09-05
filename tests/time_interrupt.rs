use lunacy::{
    Errno,
    time::{Timespec, nanosleep},
};

extern "C" fn alarm_handler(_: libc::c_int) {}

// A separate, single-threaded executable keeps SIGALRM out of other tests.
fn main() {
    // SAFETY: These native structures are initialized before use. The handler
    // does nothing, and no other thread changes signal dispositions or alarms.
    unsafe {
        let mut action: libc::sigaction = core::mem::zeroed();
        let mut previous: libc::sigaction = core::mem::zeroed();
        action.sa_sigaction = alarm_handler as *const () as usize;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGALRM, &action, &mut previous), 0);
        let request = Timespec {
            tv_sec: 5,
            tv_nsec: 0,
        };
        let mut remaining = Timespec::default();
        libc::alarm(1);
        let result = nanosleep(&request, Some(&mut remaining));
        libc::alarm(0);
        let restored = libc::sigaction(libc::SIGALRM, &previous, core::ptr::null_mut());
        assert_eq!(restored, 0);
        assert_eq!(result, Err(Errno::EINTR));
        assert!((0..1_000_000_000).contains(&remaining.tv_nsec));
        assert!((0..=5).contains(&remaining.tv_sec));
        assert_ne!(remaining, Timespec::default());
    }
}
