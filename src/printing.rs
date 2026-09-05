use core::fmt::Arguments;

use crate::{Errno, sys};

// Public only so exported macros work in downstream crates. Only the two
// process-wide standard output channels are exposed, never arbitrary raw fds.
#[doc(hidden)]
pub fn print(arguments: Arguments<'_>, stderr: bool, newline: bool) -> Result<usize, Errno> {
    let mut message = alloc::fmt::format(arguments);
    if newline {
        message.push('\n');
    }
    // SAFETY: C selects stdout/stderr's standard descriptor. The message buffer
    // remains readable for the entire call; no BorrowedFd lifetime is fabricated.
    // Capture errno before dropping message, since deallocation may change errno.
    sys::cvt_size(unsafe { sys::lunacy_print(message.as_ptr(), message.len(), stderr.into()) })
}

/// Formats into a temporary String and calls libc write once on stdout.
///
/// Returns `Result<usize, Errno>` with the byte count or native error. A short
/// write is `Ok(n)`; `?` propagates errors but does not detect incomplete output.
/// EINTR is returned without retrying. No newline is added.
///
/// Uses Rust formatting and requires a global allocator. Formatting failures
/// and allocation failures follow `alloc::format!` behavior. I/O errors are
/// returned instead of panicking. Standard descriptors follow process-wide
/// redirection; these macros do not close or duplicate them, flush libc stdio,
/// acquire an output lock, or change signal handling (including SIGPIPE).
/// Formatting completes before the write; separate calls can interleave.
///
/// ```no_run
/// let name = "lunacy";
/// let written = lunacy::print!("hello from {name}")?;
/// # Ok::<(), lunacy::Errno>(())
/// ```
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::__print(::core::format_args!($($arg)*), false, false)
    };
}

/// Formats a message and trailing LF into a temporary String, then calls libc
/// write once on stdout. `println!()` writes just the newline.
///
/// Returns `Result<usize, Errno>`, including the newline in the byte count.
/// Short writes remain successful counts and EINTR is not retried. See
/// [`crate::print!`] for allocation, formatting, and standard-output semantics.
///
/// ```no_run
/// lunacy::println!("value = {:04x}", 42)?;
/// lunacy::println!()?;
/// # Ok::<(), lunacy::Errno>(())
/// ```
#[macro_export]
macro_rules! println {
    () => {
        $crate::__print(::core::format_args!(""), false, true)
    };
    ($($arg:tt)*) => {
        $crate::__print(::core::format_args!($($arg)*), false, true)
    };
}

/// Like [`crate::print!`], but writes to stderr.
///
/// Returns `Result<usize, Errno>`. Formats into temporary allocated storage,
/// makes one write, and preserves short writes and interruptions.
///
/// ```no_run
/// lunacy::eprint!("error: {}", "missing argument")?;
/// # Ok::<(), lunacy::Errno>(())
/// ```
#[macro_export]
macro_rules! eprint {
    ($($arg:tt)*) => {
        $crate::__print(::core::format_args!($($arg)*), true, false)
    };
}

/// Like [`crate::println!`], but writes to stderr.
///
/// Returns `Result<usize, Errno>`, counting the appended LF. `eprintln!()`
/// writes just a newline. See [`crate::print!`] for the common semantics.
///
/// ```no_run
/// lunacy::eprintln!("error: {:?}", lunacy::Errno::ENOENT)?;
/// # Ok::<(), lunacy::Errno>(())
/// ```
#[macro_export]
macro_rules! eprintln {
    () => {
        $crate::__print(::core::format_args!(""), true, true)
    };
    ($($arg:tt)*) => {
        $crate::__print(::core::format_args!($($arg)*), true, true)
    };
}
