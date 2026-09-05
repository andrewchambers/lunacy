#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{Args, Mode, OpenFlags, fstat, open, print};

mod program {
    use super::{Args, Mode, OpenFlags, fstat, open, print};

    #[cfg_attr(all(panic = "abort", feature = "macros"), lunacy::main)]
    pub(super) fn main(args: Args<'_>) -> i32 {
        let path = args.get(1).unwrap_or(c"README.md");
        let file = open(path, OpenFlags::rdonly(), Mode::empty()).expect("open failed");
        let info = fstat(file.as_fd()).expect("fstat failed");
        print!("{}: {} bytes\n", path.to_string_lossy(), info.st_size).expect("write failed");
        0
    }
}

#[cfg(all(panic = "abort", not(feature = "macros")))]
lunacy::lunacy_main!(program::main);

#[cfg(not(panic = "abort"))]
fn main() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let strings: Vec<_> = std::env::args_os()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    // SAFETY: Both storage vectors remain alive and unchanged during main.
    let args = unsafe { Args::from_raw(pointers.len(), pointers.as_ptr()) };
    std::process::exit(program::main(args));
}
