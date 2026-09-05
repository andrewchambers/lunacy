#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{
    Errno,
    args::Args,
    fs::{self, Mode, OpenFlags},
};

mod program {
    use super::*;

    fn run(args: Args<'_>) -> Result<usize, Errno> {
        let path = args.get(1).unwrap_or(c"README.md");
        let file = fs::open(path, OpenFlags::rdonly(), Mode::empty())?;
        let info = fs::fstat(file.as_fd())?;
        lunacy::println!("{}: {} bytes", path.to_string_lossy(), info.st_size)
        // file closes on drop; as_fd() only borrowed it for fstat.
    }

    pub(super) fn main(args: Args<'_>) -> i32 {
        match run(args) {
            // Printing returns the native count; a short write is still Ok(n).
            Ok(_) => 0,
            Err(error) => {
                let _ = lunacy::eprintln!("file_info: {error:?}");
                1
            }
        }
    }
}

#[cfg(panic = "abort")]
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
