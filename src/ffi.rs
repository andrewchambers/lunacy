//! Minimal C ABI bindings used by `lunacy`.
//!
//! This module intentionally binds only the libc surface the crate wraps. It is
//! not meant to cover the full platform C library.

#![allow(non_camel_case_types)]

pub use core::ffi::{c_char, c_int, c_uint, c_void};

/// C `size_t`.
pub type size_t = usize;

/// POSIX `ssize_t`.
pub type ssize_t = isize;

/// POSIX `mode_t`.
pub type mode_t = c_uint;

/// Selected file metadata copied from C `struct stat`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(C)]
pub struct Stat {
    /// Device ID.
    pub dev: i64,
    /// Inode number.
    pub ino: u64,
    /// File mode bits.
    pub mode: u32,
    /// Link count.
    pub nlink: u64,
    /// Owner user ID.
    pub uid: u32,
    /// Owner group ID.
    pub gid: u32,
    /// Device ID for special files.
    pub rdev: i64,
    /// File size in bytes.
    pub size: i64,
    /// Preferred block size for I/O.
    pub blksize: i64,
    /// Allocated block count.
    pub blocks: i64,
    /// Last access time, in seconds since the Unix epoch.
    pub atime: i64,
    /// Last modification time, in seconds since the Unix epoch.
    pub mtime: i64,
    /// Last status change time, in seconds since the Unix epoch.
    pub ctime: i64,
}

unsafe extern "C" {
    /// Allocates aligned memory.
    pub fn posix_memalign(memptr: *mut *mut c_void, align: size_t, size: size_t) -> c_int;

    /// Releases memory allocated by libc.
    pub fn free(ptr: *mut c_void);

    /// Reads bytes from a file descriptor.
    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;

    /// Writes bytes to a file descriptor.
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;

    /// Closes a file descriptor.
    pub fn close(fd: c_int) -> c_int;

    /// Terminates the process without running destructors.
    pub fn _exit(status: c_int) -> !;

    /// Duplicates a file descriptor.
    pub fn dup(fd: c_int) -> c_int;

    /// Duplicates a file descriptor onto another descriptor number.
    pub fn dup2(fd: c_int, target: c_int) -> c_int;

    /// Synchronizes a file descriptor's in-core state with storage.
    pub fn fsync(fd: c_int) -> c_int;

    /// Creates a pipe.
    pub fn pipe(fds: *mut c_int) -> c_int;

    /// Opens a path.
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;

    /// Unlinks a path.
    pub fn unlink(path: *const c_char) -> c_int;

    /// Creates a directory.
    pub fn mkdir(path: *const c_char, mode: mode_t) -> c_int;

    /// Removes a directory.
    pub fn rmdir(path: *const c_char) -> c_int;

    /// Renames a filesystem path.
    pub fn rename(old: *const c_char, new: *const c_char) -> c_int;

    /// Returns the value of an environment variable.
    pub fn getenv(name: *const c_char) -> *mut c_char;

    /// Sets or replaces an environment variable.
    pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
}
