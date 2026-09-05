#![cfg_attr(panic = "abort", no_std)]
#![cfg_attr(panic = "abort", no_main)]

use lunacy::{_exit, Args, CStrArray, Errno, WaitOptions, WaitStatus, execve, fork, waitpid};

#[cfg_attr(all(panic = "abort", feature = "macros"), lunacy::main)]
fn entry(_: Args<'_>) -> i32 {
    // Allocate argument and environment pointer arrays before forking.
    let argv = CStrArray::new(&[c"sh", c"-c", c"printf 'hello from the child\n'"]);
    let envp = CStrArray::new(&[]);

    // SAFETY: The child only calls execve with prepared storage, then _exit on
    // failure. It never allocates, formats, unwinds, or runs Rust destructors.
    let pid = match unsafe { fork() } {
        Ok(0) => {
            let _ = execve(c"/bin/sh", &argv, &envp);
            _exit(127);
        }
        Ok(pid) => pid,
        Err(error) => {
            let _ = lunacy::eprintln!("fork: {error:?}");
            return 1;
        }
    };

    let mut status = WaitStatus::default();
    loop {
        match waitpid(pid, Some(&mut status), WaitOptions::empty()) {
            Ok(_) => return status.exit_status().unwrap_or(1),
            // The caller chooses whether to retry interrupted waits.
            Err(Errno::EINTR) => continue,
            Err(error) => {
                let _ = lunacy::eprintln!("waitpid: {error:?}");
                return 1;
            }
        }
    }
}

#[cfg(all(panic = "abort", not(feature = "macros")))]
lunacy::lunacy_main!(entry);

#[cfg(not(panic = "abort"))]
fn main() {
    // SAFETY: This example ignores arguments; an empty vector needs no storage.
    let args = unsafe { Args::from_raw(0, core::ptr::null()) };
    std::process::exit(entry(args));
}
