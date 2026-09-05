#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

extern crate alloc;

use alloc::sync::Arc;
use lunacy::{
    BorrowedFd, Errno,
    pthread::{self, Mutex},
    write,
};

fn run() -> Result<(), Errno> {
    let count = Arc::new(Mutex::new(0usize)?);
    let worker_count = Arc::clone(&count);
    let worker = pthread::spawn(move || -> Result<(), Errno> {
        for _ in 0..1000 {
            *worker_count.lock()? += 1;
        }
        Ok(())
    })?;
    for _ in 0..1000 {
        *count.lock()? += 1;
    }
    worker.join()??;
    if *count.lock()? != 2000 {
        return Err(Errno::EIO);
    }

    let message = b"two threads counted to 2000\n";
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
