#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{
    AddressFamily, BorrowedFd, Errno, FdSet, MsgFlags, PollEvents, PollFd, SocketType, TimeVal,
    poll, recv, select, send, socketpair, write,
};

fn run() -> Result<(), Errno> {
    let (sender, receiver) = socketpair(AddressFamily::Unix, SocketType::Stream, 0)?;
    let message = b"hello through a socket\n";
    if send(sender.as_fd(), message, MsgFlags::empty())? != message.len() {
        return Err(Errno::EIO);
    }
    let mut fds = [PollFd::new(Some(receiver.as_fd()), PollEvents::pollin())];
    if poll(&mut fds, 1000)? != 1 || !fds[0].revents().contains(PollEvents::pollin()) {
        return Err(Errno::EIO);
    }
    let mut read = FdSet::new();
    read.insert(receiver.as_fd())?;
    if select(
        receiver.as_raw_fd() + 1,
        Some(&mut read),
        None,
        None,
        Some(&mut TimeVal::default()),
    )? != 1
    {
        return Err(Errno::EIO);
    }
    let mut buffer = [0; 64];
    let n = recv(receiver.as_fd(), &mut buffer, MsgFlags::empty())?;
    // SAFETY: This example assumes stdout is open and never closes it.
    let stdout = unsafe { BorrowedFd::borrow_raw(1) };
    if write(stdout, &buffer[..n])? != n {
        return Err(Errno::EIO);
    }
    Ok(())
}

#[cfg_attr(all(panic = "abort", feature = "macros"), lunacy::main)]
fn entry(_: lunacy::Args<'_>) -> i32 {
    match run() {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

#[cfg(all(panic = "abort", not(feature = "macros")))]
lunacy::lunacy_main!(entry);

#[cfg(not(panic = "abort"))]
fn main() {
    // SAFETY: This example ignores arguments; an empty vector needs no storage.
    let args = unsafe { lunacy::Args::from_raw(0, core::ptr::null()) };
    std::process::exit(entry(args));
}
