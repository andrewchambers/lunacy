#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{BorrowedFd, Clock, Errno, Timespec, clock_getres, clock_gettime, nanosleep, write};

fn run() -> Result<(), Errno> {
    let _wall_time = clock_gettime(Clock::Realtime)?;
    let _resolution = clock_getres(Clock::Monotonic)?;
    let before = clock_gettime(Clock::Monotonic)?;
    nanosleep(
        &Timespec {
            tv_sec: 0,
            tv_nsec: 1_000_000,
        },
        None,
    )?;
    let after = clock_gettime(Clock::Monotonic)?;
    if (after.tv_sec, after.tv_nsec) < (before.tv_sec, before.tv_nsec) {
        return Err(Errno::EIO);
    }
    let message = b"clocks and sleep work\n";
    // SAFETY: This example assumes stdout is open and never closes it.
    let stdout = unsafe { BorrowedFd::borrow_raw(1) };
    if write(stdout, message)? != message.len() {
        return Err(Errno::EIO);
    }
    Ok(())
}

fn entry(_: lunacy::Args<'_>) -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(panic = "abort")]
lunacy::lunacy_main!(entry);

#[cfg(not(panic = "abort"))]
fn main() {
    // SAFETY: This example ignores arguments; an empty vector needs no storage.
    let args = unsafe { lunacy::Args::from_raw(0, core::ptr::null()) };
    std::process::exit(entry(args));
}
