#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{
    Errno,
    env::getenv,
    fd::{BorrowedFd, close, pipe},
    io::{read, write},
};

fn run() -> Result<(), Errno> {
    let mut buffer = [0; 256];
    let message = getenv(c"LUNACY_MESSAGE");
    let bytes = message
        .as_deref()
        .unwrap_or(c"hello through a pipe\n")
        .to_bytes();
    if bytes.len() > buffer.len() {
        return Err(Errno::EINVAL);
    }
    let length = bytes.len();

    let (reader, writer) = pipe()?;
    // This bounded message fits within POSIX's minimum PIPE_BUF. The example
    // treats any short transfer as failure instead of introducing a retry loop.
    if write(writer.as_fd(), bytes)? != length {
        return Err(Errno::EIO);
    }
    close(writer)?;
    let count = read(reader.as_fd(), &mut buffer)?;
    if count != length {
        return Err(Errno::EIO);
    }
    // SAFETY: This example assumes stdout is open and never closes it.
    let stdout = unsafe { BorrowedFd::borrow_raw(1) };
    if write(stdout, &buffer[..count])? != count {
        return Err(Errno::EIO);
    }
    Ok(())
}

fn entry(_: lunacy::args::Args<'_>) -> i32 {
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
    let args = unsafe { lunacy::args::Args::from_raw(0, core::ptr::null()) };
    std::process::exit(entry(args));
}
