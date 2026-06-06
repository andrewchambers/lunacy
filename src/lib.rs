#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

//! `no_std` wrappers around system libc.
//!
//! `lunacy` keeps the raw C ABI surface it uses available through [`ffi`] and
//! layers small Rust conveniences on top without depending on `std`.

extern crate alloc;

#[cfg(test)]
extern crate std;

#[cfg(unix)]
mod allocator;
pub mod cstr;
#[cfg(unix)]
pub mod env;
pub mod errno;
#[cfg(unix)]
pub mod fd;
pub mod ffi;
#[cfg(unix)]
pub mod fs;
#[cfg(unix)]
pub mod net;
#[cfg(all(unix, feature = "pthread"))]
pub mod pthread;
pub mod runtime;
#[cfg(unix)]
pub mod time;

#[cfg(unix)]
pub use allocator::LibcAllocator;
pub use errno::{Errno, ErrnoName, Result};
