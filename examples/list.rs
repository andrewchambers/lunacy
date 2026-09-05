#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{
    Errno,
    args::Args,
    dir::{closedir, opendir, readdir},
    errno::strerror_r,
    fd::BorrowedFd,
    io::write,
};

fn run(args: &Args<'_>) -> Result<(), Errno> {
    if args.len() > 2 {
        return Err(Errno::EINVAL);
    }
    let path = args.get(1).unwrap_or(c".");
    let mut directory = opendir(path)?;
    // SAFETY: This example assumes stdout and stderr are open and never closes them.
    let stdout = unsafe { BorrowedFd::borrow_raw(1) };
    while let Some(entry) = readdir(&mut directory)? {
        let name = entry.d_name.to_bytes();
        if write(stdout, name)? != name.len() || write(stdout, b"\n")? != 1 {
            return Err(Errno::EIO);
        }
    }
    closedir(directory)
}

fn entry(args: Args<'_>) -> i32 {
    match run(&args) {
        Ok(()) => 0,
        Err(error) => {
            let mut buffer = [0; 256];
            let text = strerror_r(error, &mut buffer).unwrap_or(c"lunacy: operation failed");
            // SAFETY: This example assumes stderr is open and never closes it.
            let stderr = unsafe { BorrowedFd::borrow_raw(2) };
            let _ = write(stderr, text.to_bytes());
            let _ = write(stderr, b"\n");
            1
        }
    }
}

#[cfg(panic = "abort")]
lunacy::lunacy_main!(entry);

#[cfg(not(panic = "abort"))]
fn main() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    let strings: Vec<_> = std::env::args_os()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();
    let pointers: Vec<_> = strings.iter().map(|s| s.as_ptr()).collect();
    // SAFETY: Both storage vectors remain alive and unchanged during entry.
    let args = unsafe { Args::from_raw(pointers.len(), pointers.as_ptr()) };
    std::process::exit(entry(args));
}
