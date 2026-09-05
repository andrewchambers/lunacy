// Harness-free: forks, environment changes, and signal handlers are isolated
// from the normal threaded test harness. Every child is explicitly reaped.
use lunacy::{
    _exit, CStrArray, Errno, Pid, WaitOptions, WaitStatus, close, dup2_raw, execv, execve, execvp,
    fork, getenv, pipe, read, setenv, unsetenv, waitpid, write,
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    convert::Infallible,
    ffi::{CStr, CString},
    os::unix::{ffi::OsStrExt, fs::PermissionsExt},
    sync::atomic::{AtomicBool, Ordering},
};

static CHILD: AtomicBool = AtomicBool::new(false);
struct Allocator;

// SAFETY: Storage operations delegate to System with the original layouts.
// A Rust allocation/deallocation in the restricted child fails the test via
// _exit instead of touching potentially inherited allocator state.
unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if CHILD.load(Ordering::Relaxed) {
            _exit(120);
        }
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if CHILD.load(Ordering::Relaxed) {
            _exit(121);
        }
        unsafe { System.dealloc(pointer, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

fn new_child() -> Pid {
    // SAFETY: Child branches below perform only prepared exec, bounded I/O,
    // native signal operations, and _exit. The execvp test is single-threaded.
    let pid = unsafe { fork() }.unwrap();
    if pid == 0 {
        CHILD.store(true, Ordering::Relaxed);
    }
    pid
}

fn reap(pid: Pid) -> WaitStatus {
    let mut status = WaitStatus::default();
    loop {
        match waitpid(pid, Some(&mut status), WaitOptions::empty()) {
            Err(Errno::EINTR) => continue,
            result => {
                assert_eq!(result, Ok(pid));
                return status;
            }
        }
    }
}

fn check_exec(operation: impl FnOnce() -> Result<Infallible, Errno>, expected: i32, text: &[u8]) {
    let (reader, writer) = pipe().unwrap();
    let pid = new_child();
    if pid == 0 {
        if close(reader).is_err() {
            _exit(110);
        }
        // SAFETY: This child exclusively controls stdout and has no borrowers
        // of its previous resource. No Rust stdio is accessed before exec.
        if unsafe { dup2_raw(writer.as_fd(), libc::STDOUT_FILENO) }.is_err() {
            _exit(111);
        }
        let _ = operation();
        _exit(112);
    }
    drop(writer);
    let status = reap(pid);
    assert_eq!(
        status.exit_status(),
        Some(expected),
        "native status {}",
        status.to_raw()
    );
    assert_eq!(status.terminating_signal(), None);
    assert_eq!(status.stopping_signal(), None);
    assert!(!status.continued());
    let mut bytes = [0; 128];
    let mut count = 0;
    loop {
        match read(reader.as_fd(), &mut bytes[count..]) {
            Ok(0) => break,
            Ok(n) => count += n,
            Err(Errno::EINTR) => continue,
            Err(error) => panic!("read failed: {error:?}"),
        }
    }
    assert_eq!(&bytes[..count], text);
}

fn arrays_and_errors() {
    let owned = CString::new(b"non-utf8-\xff".to_vec()).unwrap();
    let argv = CStrArray::new(&[c"chosen-zero", c"", &owned]);
    assert_eq!(argv.len(), 3);
    assert!(!argv.is_empty());
    // SAFETY: The array owns four initialized pointers; non-null entries borrow
    // the strings still alive in this scope.
    unsafe {
        assert_eq!(CStr::from_ptr(*argv.as_ptr().add(1)), c"");
        assert_eq!(CStr::from_ptr(*argv.as_ptr().add(2)), owned.as_c_str());
        assert!((*argv.as_ptr().add(3)).is_null());
    }
    let empty = CStrArray::new(&[]);
    assert!(empty.is_empty());
    assert!(unsafe { *empty.as_ptr() }.is_null());
    let missing = c"/lunacy-exec-test-path-that-does-not-exist/program";
    assert_eq!(execv(missing, &argv), Err(Errno::ENOENT));
    assert_eq!(execve(missing, &argv, &empty), Err(Errno::ENOENT));
    assert_eq!(execvp(missing, &argv), Err(Errno::ENOENT));
    assert_eq!(argv.len(), 3); // Failed exec leaves borrowed inputs usable.
    for (error, native) in [
        (Errno::E2BIG, libc::E2BIG),
        (Errno::ECHILD, libc::ECHILD),
        (Errno::ENOEXEC, libc::ENOEXEC),
        (Errno::ETXTBSY, libc::ETXTBSY),
    ] {
        assert_eq!(error.to_raw(), native);
        assert_eq!(Errno::from_raw(native), error);
    }
}

fn exec_arguments_and_environments() {
    let executable = CString::new(std::env::current_exe().unwrap().as_os_str().as_bytes()).unwrap();
    // SAFETY: This dedicated process is single-threaded with no borrowed values
    // referring to environment storage. All children are reaped before mutation.
    unsafe {
        setenv(c"LUNACY_EXEC_TEST", c"inherited", true).unwrap();
    }
    let argv = CStrArray::new(&[
        c"chosen-zero",
        c"--exec-probe",
        c"",
        c"non-utf8-\xff",
        c"two words",
        c"inherited",
    ]);
    check_exec(|| execv(&executable, &argv), 37, b"exec ok\n");

    let argv = CStrArray::new(&[
        c"chosen-zero",
        c"--exec-probe",
        c"",
        c"non-utf8-\xff",
        c"two words",
        c"explicit",
    ]);
    let envp = CStrArray::new(&[c"LUNACY_EXEC_TEST=explicit"]);
    check_exec(|| execve(&executable, &argv, &envp), 37, b"exec ok\n");
    assert_eq!(getenv(c"LUNACY_EXEC_TEST").as_deref(), Some(c"inherited"));

    let argv = CStrArray::new(&[
        c"chosen-zero",
        c"--exec-probe",
        c"",
        c"non-utf8-\xff",
        c"two words",
        c"empty",
    ]);
    let empty = CStrArray::new(&[]);
    check_exec(|| execve(&executable, &argv, &empty), 37, b"exec ok\n");
    unsafe {
        unsetenv(c"LUNACY_EXEC_TEST").unwrap();
    }
}

fn path_search_and_shell_fallback() {
    let directory = std::env::temp_dir().join(format!("lunacy-exec-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let script = directory.join("lunacy-probe");
    std::fs::write(&script, b"exit 23\n").unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    let path = CString::new(script.as_os_str().as_bytes()).unwrap();
    let argv = CStrArray::new(&[c"lunacy-probe"]);
    assert_eq!(execv(&path, &argv), Err(Errno::ENOEXEC));
    let saved = getenv(c"PATH");
    let search = CString::new(directory.as_os_str().as_bytes()).unwrap();
    // SAFETY: Still single-threaded, with independent environment copies.
    unsafe {
        setenv(c"PATH", &search, true).unwrap();
    }
    check_exec(|| execvp(c"lunacy-probe", &argv), 23, b"");
    // A slash bypasses PATH lookup, including the same native shell fallback.
    check_exec(|| execvp(&path, &argv), 23, b"");
    unsafe {
        match saved.as_deref() {
            Some(value) => setenv(c"PATH", value, true).unwrap(),
            None => unsetenv(c"PATH").unwrap(),
        }
    }
    std::fs::remove_dir_all(directory).unwrap();
}

fn wait_status_and_nohang() {
    assert_eq!(WaitOptions::nohang().to_raw(), libc::WNOHANG);
    assert_eq!(WaitOptions::untraced().to_raw(), libc::WUNTRACED);
    let mut options = WaitOptions::nohang() | WaitOptions::untraced();
    options |= WaitOptions::continued();
    assert_eq!(
        options.to_raw(),
        libc::WNOHANG | libc::WUNTRACED | libc::WCONTINUED
    );
    let (reader, writer) = pipe().unwrap();
    let pid = new_child();
    if pid == 0 {
        let _ = close(writer);
        let mut buffer = [0];
        if read(reader.as_fd(), &mut buffer) != Ok(1) {
            _exit(113);
        }
        _exit(0x123);
    }
    let sentinel = WaitStatus::from_raw(0x1234);
    let mut status = sentinel;
    assert_eq!(
        waitpid(pid, Some(&mut status), WaitOptions::nohang()),
        Ok(0)
    );
    assert_eq!(status, sentinel);
    write(writer.as_fd(), b"x").unwrap();
    status = reap(pid);
    assert_eq!(status.exit_status(), Some(0x23));
    assert_eq!(libc::WEXITSTATUS(status.to_raw()), 0x23);
    assert!(libc::WIFEXITED(status.to_raw()));
    status = sentinel;
    assert_eq!(
        waitpid(pid, Some(&mut status), WaitOptions::empty()),
        Err(Errno::ECHILD)
    );
    assert_eq!(status, sentinel);
    if size_of::<libc::pid_t>() < size_of::<Pid>() {
        assert_eq!(
            waitpid(i64::MAX, Some(&mut status), WaitOptions::empty()),
            Err(Errno::EOVERFLOW)
        );
        assert_eq!(status, sentinel);
    }

    let pid = new_child();
    if pid == 0 {
        _exit(0);
    }
    assert_eq!(waitpid(pid, None, WaitOptions::empty()), Ok(pid));
}

fn stopped_continued_and_signaled() {
    let (reader, writer) = pipe().unwrap();
    let pid = new_child();
    if pid == 0 {
        let _ = close(writer);
        // SAFETY: These signal operations affect only this child.
        unsafe {
            libc::raise(libc::SIGSTOP);
        }
        let mut byte = [0];
        let _ = read(reader.as_fd(), &mut byte); // Stay alive after SIGCONT.
        _exit(114);
    }
    let mut status = WaitStatus::default();
    assert_eq!(
        waitpid(pid, Some(&mut status), WaitOptions::untraced()),
        Ok(pid)
    );
    assert_eq!(status.stopping_signal(), Some(libc::SIGSTOP));
    assert_eq!(status.exit_status(), None);
    assert_eq!(status.terminating_signal(), None);
    let native_pid = libc::pid_t::try_from(pid).unwrap();
    // SAFETY: pid identifies our live child, which is always reaped below.
    assert_eq!(unsafe { libc::kill(native_pid, libc::SIGCONT) }, 0);
    assert_eq!(
        waitpid(pid, Some(&mut status), WaitOptions::continued()),
        Ok(pid)
    );
    assert!(status.continued());
    assert_eq!(status.exit_status(), None);
    assert_eq!(unsafe { libc::kill(native_pid, libc::SIGKILL) }, 0);
    status = reap(pid);
    assert_eq!(status.terminating_signal(), Some(libc::SIGKILL));
    assert_eq!(status.stopping_signal(), None);
    assert_eq!(status.exit_status(), None);
}

static ALARM_FIRED: AtomicBool = AtomicBool::new(false);

extern "C" fn alarm_handler(_: libc::c_int) {
    if ALARM_FIRED.swap(true, Ordering::Relaxed) {
        _exit(119); // Bound a broken automatic retry instead of hanging forever.
    }
    // SAFETY: alarm is async-signal-safe; this process owns the alarm timer.
    unsafe {
        libc::alarm(2);
    }
}

fn interrupted_wait() {
    let (reader, writer) = pipe().unwrap();
    let pid = new_child();
    if pid == 0 {
        let _ = close(writer);
        let mut byte = [0];
        let _ = read(reader.as_fd(), &mut byte);
        _exit(0);
    }
    // SAFETY: This harness-free process exclusively owns the signal disposition.
    unsafe {
        let mut action: libc::sigaction = core::mem::zeroed();
        let mut previous: libc::sigaction = core::mem::zeroed();
        action.sa_sigaction = alarm_handler as *const () as usize;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGALRM, &action, &mut previous), 0);
        let sentinel = WaitStatus::from_raw(0x1234);
        let mut status = sentinel;
        libc::alarm(1);
        let result = waitpid(pid, Some(&mut status), WaitOptions::empty());
        libc::alarm(0);
        assert_eq!(
            libc::sigaction(libc::SIGALRM, &previous, core::ptr::null_mut()),
            0
        );
        write(writer.as_fd(), b"x").unwrap();
        assert_eq!(reap(pid).exit_status(), Some(0));
        assert_eq!(result, Err(Errno::EINTR));
        assert_eq!(status, sentinel);
    }
}

fn multithreaded_fork_execve() {
    let argv = CStrArray::new(&[c"sh", c"-c", c"exit 19"]);
    let envp = CStrArray::new(&[]);
    let (sender, receiver) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || receiver.recv().unwrap());
    // Prepared execve plus _exit only in the child; no heap use on either the
    // success or failure path. The allocator above checks Rust heap accesses.
    check_exec(|| execve(c"/bin/sh", &argv, &envp), 19, b"");
    let missing = c"/lunacy-exec-test-path-that-does-not-exist/program";
    let pid = new_child();
    if pid == 0 {
        if execve(missing, &argv, &envp) == Err(Errno::ENOENT) {
            _exit(17);
        }
        _exit(115);
    }
    let status = reap(pid);
    sender.send(()).unwrap();
    thread.join().unwrap();
    assert_eq!(status.exit_status(), Some(17));
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_some_and(|arg| arg == "--exec-probe") {
        assert_eq!(args.len(), 6);
        assert_eq!(args[0], "chosen-zero");
        assert_eq!(args[2], "");
        assert_eq!(args[3].as_bytes(), b"non-utf8-\xff");
        assert_eq!(args[4], "two words");
        if args[5] == "empty" {
            assert_eq!(std::env::vars_os().count(), 0);
        } else {
            assert_eq!(
                std::env::var_os("LUNACY_EXEC_TEST").as_ref(),
                Some(&args[5])
            );
        }
        // SAFETY: Static bytes are readable and stdout is the inherited pipe.
        assert_eq!(
            unsafe { libc::write(1, b"exec ok\n".as_ptr().cast(), 8) },
            8
        );
        _exit(37);
    }
    arrays_and_errors();
    exec_arguments_and_environments();
    path_search_and_shell_fallback();
    wait_status_and_nohang();
    stopped_continued_and_signaled();
    interrupted_wait();
    multithreaded_fork_execve();
    println!("process checks passed");
}
