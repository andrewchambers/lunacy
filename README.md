# lunacy

`lunacy` is a `#![no_std]` Rust library for wrapping system libc while still
being usable with `alloc`.

The initial API is intentionally small:

- `lunacy::ffi` exposes the raw C ABI surface used by this crate.
- `Errno` and `Result<T>` avoid `std::io::Error`.
- Errno values used by wrappers are read through C helpers, not Rust constants.
- `ErrnoName` provides symbolic names such as `EINTR` and `ENOENT`, with
  runtime mapping through C for libcs with dynamic errno values.
- Unix environment helpers wrap `getenv` and unsafe `setenv`.
- Unix file descriptor helpers accept `RawFd`, `BorrowedFd`, and references to
  `OwnedFd`.
- Safe Unix I/O helpers wrap `pipe`, `dup`, `dup2`, `read`, `write`,
  `write_all`, `close`, `fsync`, `lseek`, `poll`, `select`, and common
  descriptor flags.
- Unix standard fd constants expose `STDIN`, `STDOUT`, and `STDERR`.
- Unix path helpers wrap `open`, `open_mode`, `stat`, `fstat`, `mkdir`,
  `rmdir`, `rename`, and `unlink`; open calls return `OwnedFd`.
- Unix networking helpers wrap `socket`, `connect`, `bind`, `listen`, `accept`,
  `shutdown`, `send`, `recv`, `sendto`, and `recvfrom` with C-backed constants
  and IPv4/IPv6 address conversion.
- Unix time helpers wrap `time`, `clock_gettime`, `gettimeofday`, and
  `nanosleep`, including a retrying `sleep`.
- `LibcAllocator` can be used as a no_std global allocator backed by libc.
- With the `pthread` feature, `lunacy::pthread` provides heap-backed opaque
  pthread handles for `Thread` and `Mutex<T>`.

```rust
#![no_std]

extern crate alloc;

use lunacy::LibcAllocator;

#[global_allocator]
static ALLOCATOR: LibcAllocator = LibcAllocator;
```

This crate assumes the target has a system libc. It is not meant for bare-metal
targets that lack libc.

`lunacy` builds a small C shim for locating `errno`, so cross-compilation needs
a C compiler and libc headers for the target.
