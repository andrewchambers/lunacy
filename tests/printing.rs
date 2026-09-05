use lunacy::Errno;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ffi::{CStr, CString},
    fs::File,
    io::{Read, Seek, SeekFrom},
    os::{
        fd::{AsRawFd, FromRawFd, OwnedFd},
        unix::{ffi::OsStrExt, net::UnixStream},
    },
    sync::atomic::{AtomicBool, Ordering},
};

static CLOBBER_ERRNO: AtomicBool = AtomicBool::new(false);
struct TestAllocator;
// SAFETY: All memory operations are delegated to System with the original layout.
unsafe impl GlobalAlloc for TestAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        unsafe { System.alloc(layout) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        if CLOBBER_ERRNO.load(Ordering::Relaxed) {
            // Deliberately set EBADF during destruction of the formatted message.
            unsafe { libc::close(-1) };
        }
    }
}
#[global_allocator]
static ALLOCATOR: TestAllocator = TestAllocator;

struct Redirect {
    target: libc::c_int,
    saved: OwnedFd,
}
impl Redirect {
    fn new(target: libc::c_int, source: libc::c_int) -> Self {
        // SAFETY: This executable has no concurrent standard-I/O users. dup
        // creates an owned descriptor; dup2 replaces only our standard channel.
        let saved = unsafe { libc::dup(target) };
        assert!(saved >= 0);
        let saved = unsafe { OwnedFd::from_raw_fd(saved) };
        assert_eq!(unsafe { libc::dup2(source, target) }, target);
        Self { target, saved }
    }
}
impl Drop for Redirect {
    fn drop(&mut self) {
        // SAFETY: saved remains open; restore the standard channel before close.
        assert_eq!(
            unsafe { libc::dup2(self.saved.as_raw_fd(), self.target) },
            self.target
        );
    }
}

fn temporary_file() -> File {
    let template = std::env::temp_dir().join("lunacy-print-XXXXXX");
    let mut template = CString::new(template.as_os_str().as_bytes())
        .unwrap()
        .into_bytes_with_nul();
    let fd = lunacy::mkstemp(&mut template).unwrap();
    lunacy::unlink(CStr::from_bytes_with_nul(&template).unwrap()).unwrap();
    // SAFETY: into_raw_fd transfers exclusive ownership to File.
    unsafe { File::from_raw_fd(fd.into_raw_fd()) }
}

fn capture<T>(target: libc::c_int, f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let mut file = temporary_file();
    let redirect = Redirect::new(target, file.as_raw_fd());
    let value = f();
    drop(redirect);
    file.seek(SeekFrom::Start(0)).unwrap();
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).unwrap();
    (value, bytes)
}

fn formatting_and_channels() {
    let (((), err), out) = capture(libc::STDOUT_FILENO, || {
        capture(libc::STDERR_FILENO, || {
            let name = "lunacy";
            assert_eq!(lunacy::print!("hello {name}"), Ok(12));
            assert_eq!(lunacy::println!(" {:04x}", 42,), Ok(6));
            assert_eq!(lunacy::println!(), Ok(1));
            assert_eq!(lunacy::print!(""), Ok(0));
            assert_eq!(lunacy::eprint!("error: {code}", code = 7), Ok(8));
            assert_eq!(lunacy::eprintln!(), Ok(1));
            assert_eq!(lunacy::eprintln!("{}", "é"), Ok(3));
            assert_eq!(lunacy::eprint!(""), Ok(0));
            let mut evaluations = 0;
            assert_eq!(
                lunacy::print!("{}", {
                    evaluations += 1;
                    evaluations
                }),
                Ok(1)
            );
            assert_eq!(evaluations, 1);
            assert_eq!(lunacy::println!(" {}", String::from("temporary")), Ok(11));
        })
    });
    assert_eq!(out, b"hello lunacy 002a\n\n1 temporary\n");
    assert_eq!(err, "error: 7\né\n".as_bytes());
}

fn native_errors_survive_message_deallocation() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let stdout = Redirect::new(libc::STDOUT_FILENO, full.as_raw_fd());
    let stderr = Redirect::new(libc::STDERR_FILENO, full.as_raw_fd());
    CLOBBER_ERRNO.store(true, Ordering::Relaxed);
    let results = [
        lunacy::print!("full"),
        lunacy::println!("full"),
        lunacy::eprint!("full"),
        lunacy::eprintln!("full"),
    ];
    CLOBBER_ERRNO.store(false, Ordering::Relaxed);
    drop(stderr);
    drop(stdout);
    for result in results {
        assert_eq!(result, Err(Errno::ENOSPC));
    }

    let file = temporary_file();
    let stdout = Redirect::new(libc::STDOUT_FILENO, file.as_raw_fd());
    // SAFETY: No other thread or borrowed descriptor refers to this standard slot.
    assert_eq!(unsafe { libc::close(libc::STDOUT_FILENO) }, 0);
    let result = lunacy::print!("closed");
    drop(stdout);
    assert_eq!(result, Err(Errno::EBADF));
}

fn short_write_is_returned_without_completing_or_retrying() {
    let (writer, mut reader) = UnixStream::pair().unwrap();
    writer.set_nonblocking(true).unwrap();
    let stdout = Redirect::new(libc::STDOUT_FILENO, writer.as_raw_fd());
    let payload = "x".repeat(4 * 1024 * 1024);
    let first = lunacy::println!("{payload}");
    let second = lunacy::print!("{payload}");
    drop(stdout);
    drop(writer);
    let count = first.unwrap();
    assert!(count > 0 && count < payload.len());
    assert_eq!(second, Err(Errno::EAGAIN));
    let mut actual = Vec::new();
    reader.read_to_end(&mut actual).unwrap();
    assert_eq!(actual, payload.as_bytes()[..count]);
}

static ALARM_FIRED: AtomicBool = AtomicBool::new(false);
extern "C" fn interrupt(_: libc::c_int) {
    if ALARM_FIRED.swap(true, Ordering::Relaxed) {
        // Stop a broken retry loop rather than hanging the test indefinitely.
        unsafe { libc::_exit(77) };
    }
    // alarm and _exit are async-signal-safe. No allocation or I/O in the handler.
    unsafe { libc::alarm(2) };
}

fn interrupted_write_is_returned_without_retrying() {
    let (_reader, writer) = lunacy::pipe().unwrap();
    let fd = writer.as_raw_fd();
    // SAFETY: The descriptor is live and these fcntl commands take integer flags.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    assert!(flags >= 0);
    assert_eq!(
        unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) },
        0
    );
    // Fill the pipe, including any final space smaller than the first buffer.
    let buffer = [0; 4096];
    for size in [buffer.len(), 1] {
        loop {
            // SAFETY: buffer is readable for size bytes and fd is open.
            let rc = unsafe { libc::write(fd, buffer.as_ptr().cast(), size) };
            if rc == -1 {
                assert_eq!(Errno::last(), Errno::EAGAIN);
                break;
            }
            assert!(rc > 0);
        }
    }
    assert_eq!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) }, 0);
    let stdout = Redirect::new(libc::STDOUT_FILENO, fd);
    // SAFETY: Native signal storage is initialized before use. This process has
    // no other signal-disposition users; restore the handler after the write.
    let (result, restored) = unsafe {
        let mut action: libc::sigaction = core::mem::zeroed();
        let mut previous: libc::sigaction = core::mem::zeroed();
        action.sa_sigaction = interrupt as *const () as usize;
        assert_eq!(libc::sigemptyset(&mut action.sa_mask), 0);
        assert_eq!(libc::sigaction(libc::SIGALRM, &action, &mut previous), 0);
        libc::alarm(1);
        let result = lunacy::print!("blocked");
        libc::alarm(0);
        let restored = libc::sigaction(libc::SIGALRM, &previous, core::ptr::null_mut());
        (result, restored)
    };
    drop(stdout);
    assert_eq!(restored, 0);
    assert_eq!(result, Err(Errno::EINTR));
}

fn main() {
    formatting_and_channels();
    native_errors_survive_message_deallocation();
    short_write_is_returned_without_completing_or_retrying();
    interrupted_write_is_returned_without_retrying();
    std::println!("printing checks passed");
}
