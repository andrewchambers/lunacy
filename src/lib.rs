#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]
//! A small Rust interface to system libc.
//!
//! Read/write calls preserve libc's behavior without buffering or retry loops.
//! Printing macros format into temporary allocated storage before one write.
//! The allocator and executable entry point are opt-in.

extern crate alloc;

pub mod allocator;
pub mod args;
pub mod dir;
pub mod env;
pub mod errno;
pub mod fd;
pub mod fs;
pub mod io;
pub mod poll;
mod printing;
#[cfg(feature = "pthread")]
pub mod pthread;
mod runtime;
pub mod select;
pub mod socket;
#[cfg(feature = "pthread")]
pub mod sync;
mod sys;
pub mod time;

pub use errno::Errno;

#[doc(hidden)]
pub use runtime::abort;

#[doc(hidden)]
pub use printing::print as __print;
