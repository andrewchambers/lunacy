#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

extern crate alloc;

use lunacy::args::Args;

mod program {
    use super::*;

    pub(super) fn main(args: Args<'_>) -> i32 {
        let name = args.get(1).unwrap_or(c"lunacy");
        let name = name.to_string_lossy();
        match lunacy::println!("hello from {name}") {
            Ok(n) if n == "hello from ".len() + name.len() + 1 => 0,
            // Handling a short write or retrying is the application's decision.
            _ => 1,
        }
    }
}

#[cfg(panic = "abort")]
lunacy::lunacy_main!(program::main);

// Cargo's test profile forces unwinding. Let it compile the same example body
// with std; normal dev/release builds exercise the no_std entry and allocator.
#[cfg(not(panic = "abort"))]
fn main() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let strings: Vec<_> = std::env::args_os()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    // SAFETY: Both storage vectors remain alive and unchanged during program::main.
    let args = unsafe { Args::from_raw(pointers.len(), pointers.as_ptr()) };
    std::process::exit(program::main(args));
}
