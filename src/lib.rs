#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
//! A small Rust interface to system libc.
//!
//! Read/write calls preserve libc's behavior without buffering or retry loops.
//! Printing macros format into temporary allocated storage before one write.
//! The allocator and executable entry point are opt-in. Libc operations and
//! their types are available at the crate root; threads and mutexes live in
//! `pthread` when the `pthread` feature is enabled (the default).
//! The `macros` feature (also enabled by default) adds `#[lunacy::main]` for
//! executable startup.

extern crate alloc;

mod allocator;
mod args;
mod dir;
mod env;
mod errno;
mod fd;
mod fs;
mod io;
mod poll;
mod printing;
mod process;
#[cfg(feature = "pthread")]
pub mod pthread;
mod runtime;
mod select;
mod socket;
#[cfg(feature = "pthread")]
mod sync;
mod sys;
mod time;

pub use allocator::LibcAllocator;
pub use args::Args;
pub use dir::{Dir, DirEntry, closedir, opendir, readdir};
pub use env::{getenv, getenv_borrowed, setenv, unsetenv};
pub use errno::{Errno, strerror_r};
pub use fd::{BorrowedFd, OwnedFd, RawFd, close, dup, dup2, dup2_raw, pipe};
pub use fs::{
    FileType, Mode, OpenFlags, Stat, Whence, chdir, fstat, ftruncate, getcwd, isatty, lseek, lstat,
    mkdir, mkstemp, open, openat, rename, rmdir, stat, unlink,
};
pub use io::{read, write};
pub use poll::{PollEvents, PollFd, poll};
pub use process::{
    _exit, CStrArray, Pid, WaitOptions, WaitStatus, execv, execve, execvp, fork, waitpid,
};
pub use select::{FdSet, TimeVal, select};
pub use socket::{
    AddressFamily, MsgFlags, Shutdown, SockAddr, SocketType, accept, bind, connect, getpeername,
    getsockname, listen, recv, send, shutdown, socket, socketpair,
};
pub use time::{Clock, Timespec, clock_getres, clock_gettime, nanosleep};

#[cfg(feature = "macros")]
pub use lunacy_macros::main;

#[doc(hidden)]
pub use runtime::abort;

#[doc(hidden)]
pub use printing::print as __print;
