//! Process creation, replacement, and waiting, with libc's native behavior.

use alloc::vec::Vec;
use core::{
    convert::Infallible,
    ffi::{CStr, c_char, c_int},
    marker::PhantomData,
    ops::{BitOr, BitOrAssign},
    ptr,
};

use crate::Errno;

/// A process ID or waitpid selector, converted to native pid_t by the C shim.
pub type Pid = i64;

/// A pointer array terminated by a null pointer, borrowing C strings for exec
/// arguments or environment entries. Only the array is allocated; strings are not
/// copied or decoded. Construct it before fork when the child cannot allocate.
///
/// The strings remain borrowed for the array's lifetime:
/// ```compile_fail
/// let text = std::ffi::CString::new("argument").unwrap();
/// let args = lunacy::CStrArray::new(&[text.as_c_str()]);
/// drop(text);
/// let _ = lunacy::execv(c"/bin/echo", &args);
/// ```
pub struct CStrArray<'a> {
    pointers: Vec<*const c_char>,
    _strings: PhantomData<&'a CStr>,
}

impl<'a> CStrArray<'a> {
    /// Borrows the strings and allocates a pointer array with a final null.
    /// Empty input produces an array containing only the terminator. Allocation
    /// failure follows alloc's usual behavior. Do not construct or drop this
    /// array in a fork child restricted to async-signal-safe operations.
    pub fn new(strings: &[&'a CStr]) -> Self {
        let mut pointers = Vec::with_capacity(strings.len() + 1);
        pointers.extend(strings.iter().map(|string| string.as_ptr()));
        pointers.push(ptr::null());
        Self {
            pointers,
            _strings: PhantomData,
        }
    }

    /// Returns the string count, excluding the terminating null pointer.
    pub fn len(&self) -> usize {
        self.pointers.len() - 1
    }

    /// Whether the array contains no strings.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Borrows the native pointer array. Neither it nor its strings may be
    /// modified; the pointer must not outlive this array or its borrowed strings.
    pub fn as_ptr(&self) -> *const *const c_char {
        self.pointers.as_ptr()
    }
}

unsafe extern "C" {
    fn lunacy_fork() -> Pid;
    fn lunacy_execv(path: *const c_char, argv: *const *const c_char) -> c_int;
    fn lunacy_execve(
        path: *const c_char,
        argv: *const *const c_char,
        envp: *const *const c_char,
    ) -> c_int;
    fn lunacy_execvp(file: *const c_char, argv: *const *const c_char) -> c_int;
    fn lunacy_exit_immediately(status: c_int) -> !;
    fn lunacy_waitpid(pid: Pid, status: *mut c_int, options: c_int) -> Pid;
    fn lunacy_wnohang() -> c_int;
    fn lunacy_wuntraced() -> c_int;
    fn lunacy_wcontinued() -> c_int;
    fn lunacy_wifexited(status: c_int) -> c_int;
    fn lunacy_wexitstatus(status: c_int) -> c_int;
    fn lunacy_wifsignaled(status: c_int) -> c_int;
    fn lunacy_wtermsig(status: c_int) -> c_int;
    fn lunacy_wifstopped(status: c_int) -> c_int;
    fn lunacy_wstopsig(status: c_int) -> c_int;
    fn lunacy_wifcontinued(status: c_int) -> c_int;
}

/// Calls fork once: returns zero in the child and its PID in the parent.
/// Errors are returned without retrying. Inherited descriptors retain their
/// flags and share open file descriptions, including file offsets.
///
/// # Safety
/// The caller must uphold the invariants of all inherited Rust and foreign
/// state, including any registered pthread_atfork callbacks. After forking a
/// multithreaded process, the child may only perform async-signal-safe operations
/// until exec or [`_exit`]. In particular, it must not allocate, free, format,
/// lock inherited mutexes, or unwind/drop arbitrary Rust values. Prepare all
/// argument/environment arrays before the call and avoid `?` or panicking in
/// the child if that could run destructors. Signal handlers must obey the same
/// restrictions. Even a single-threaded caller must account for foreign state
/// and resources whose invariants cannot be duplicated across processes.
pub unsafe fn fork() -> Result<Pid, Errno> {
    // SAFETY: The caller accepts the invariants of continuing in both processes.
    pid_result(unsafe { lunacy_fork() })
}

/// Replaces the process using an explicit path and the inherited environment.
/// `argv` includes `argv[0]`; no program name is inserted. No PATH search occurs.
/// Calls libc execv once without allocating pointer storage or retrying.
///
/// Success never returns and runs no Rust destructors. Failure returns the
/// native error, leaving all borrowed inputs usable. Unsafe environment mutation
/// must exclude this reader. See [`fork`] for restrictions on child code.
pub fn execv(path: &CStr, argv: &CStrArray<'_>) -> Result<Infallible, Errno> {
    // SAFETY: Path and every array entry are terminated and remain borrowed.
    // POSIX exec does not modify the argument array or its strings.
    unsafe { lunacy_execv(path.as_ptr(), argv.as_ptr()) };
    Err(Errno::last())
}

/// Replaces the process using an explicit path, arguments, and environment.
/// `argv` includes `argv[0]`; `envp` contains complete `NAME=value` entries. An
/// empty environment array requests an empty environment. No entries are added,
/// validated, or inherited, and PATH is not searched.
///
/// Calls libc execve once, with no Rust/shim allocation or retry. Prepared
/// arrays make this suitable for the restricted child path described by
/// [`fork`]. Success never returns or runs destructors; failure returns errno
/// immediately and leaves the borrowed inputs intact.
pub fn execve(
    path: &CStr,
    argv: &CStrArray<'_>,
    envp: &CStrArray<'_>,
) -> Result<Infallible, Errno> {
    // SAFETY: All strings/arrays are terminated, readable, and remain borrowed.
    unsafe { lunacy_execve(path.as_ptr(), argv.as_ptr(), envp.as_ptr()) };
    Err(Errno::last())
}

/// Replaces the process, searching the inherited PATH when `file` has no slash.
/// Inherits the environment and preserves libc's search and shell-fallback
/// behavior for executable files with an unrecognized format. A slash bypasses
/// PATH search. `argv` includes `argv[0]`; it is not synthesized from `file`.
///
/// Calls libc execvp once without building new argument storage or retrying.
/// libc's search may allocate; execvp is not guaranteed async-signal-safe and
/// must not be used in a restricted child after a multithreaded fork. Unsafe
/// environment mutation must exclude this reader. Success never returns or
/// runs destructors; failure returns errno and leaves the inputs intact.
pub fn execvp(file: &CStr, argv: &CStrArray<'_>) -> Result<Infallible, Errno> {
    // SAFETY: The filename and complete, terminated argument array stay live.
    unsafe { lunacy_execvp(file.as_ptr(), argv.as_ptr()) };
    Err(Errno::last())
}

/// Calls libc _exit to terminate the process without Rust destructors, atexit
/// callbacks, or stdio flushing. The parent receives the low eight status bits.
/// This is suitable for ending a fork child after exec fails. Closing native
/// descriptors during process termination may still block.
pub fn _exit(status: c_int) -> ! {
    // SAFETY: _exit has no pointer arguments and does not return.
    unsafe { lunacy_exit_immediately(status) }
}

/// Native waitpid option bits, resolved using the target's C headers.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WaitOptions(c_int);

impl WaitOptions {
    /// No options: block until a selected child changes state.
    pub const fn empty() -> Self {
        Self(0)
    }
    /// Wraps native option bits, including platform-specific options.
    pub const fn from_raw(raw: c_int) -> Self {
        Self(raw)
    }
    /// Returns the native option bits.
    pub const fn to_raw(self) -> c_int {
        self.0
    }
    /// WNOHANG: return zero if no selected child has a reportable state change.
    pub fn nohang() -> Self {
        // SAFETY: Reads a native constant.
        Self(unsafe { lunacy_wnohang() })
    }
    /// WUNTRACED: also report stopped children.
    pub fn untraced() -> Self {
        // SAFETY: Reads a native constant.
        Self(unsafe { lunacy_wuntraced() })
    }
    /// WCONTINUED: also report children resumed by SIGCONT.
    pub fn continued() -> Self {
        // SAFETY: Reads a native constant.
        Self(unsafe { lunacy_wcontinued() })
    }
}

impl BitOr for WaitOptions {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}
impl BitOrAssign for WaitOptions {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Native wait status, interpreted by the target's wait-status macros.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WaitStatus(c_int);

impl WaitStatus {
    /// Wraps a native status value for interoperability.
    pub const fn from_raw(raw: c_int) -> Self {
        Self(raw)
    }
    /// Returns the native status bits.
    pub const fn to_raw(self) -> c_int {
        self.0
    }
    /// Returns WEXITSTATUS only when WIFEXITED is true.
    pub fn exit_status(self) -> Option<c_int> {
        // SAFETY: Native macros inspect integer bits; extraction is conditional.
        unsafe { (lunacy_wifexited(self.0) != 0).then(|| lunacy_wexitstatus(self.0)) }
    }
    /// Returns WTERMSIG only when WIFSIGNALED is true.
    pub fn terminating_signal(self) -> Option<c_int> {
        // SAFETY: Native macros inspect integer bits; extraction is conditional.
        unsafe { (lunacy_wifsignaled(self.0) != 0).then(|| lunacy_wtermsig(self.0)) }
    }
    /// Returns WSTOPSIG only when WIFSTOPPED is true.
    pub fn stopping_signal(self) -> Option<c_int> {
        // SAFETY: Native macros inspect integer bits; extraction is conditional.
        unsafe { (lunacy_wifstopped(self.0) != 0).then(|| lunacy_wstopsig(self.0)) }
    }
    /// Whether WIFCONTINUED reports a child resumed by SIGCONT.
    pub fn continued(self) -> bool {
        // SAFETY: The native macro only inspects integer bits.
        unsafe { lunacy_wifcontinued(self.0) != 0 }
    }
}

/// Calls waitpid once and returns the PID whose status was obtained, or zero
/// with WNOHANG when no selected child has a reportable state change.
///
/// Select a child with a positive PID, any child with -1, the current process
/// group with zero, or a process group with a value below -1. Values that do not
/// fit pid_t return EOVERFLOW. Status is written only for a positive result;
/// `None` discards it. EINTR is returned without retrying. Waiting for a child's
/// termination reaps it; coordinate with other threads/libraries waiting on
/// the same children. No child is reaped automatically by lunacy.
pub fn waitpid(
    pid: Pid,
    status: Option<&mut WaitStatus>,
    options: WaitOptions,
) -> Result<Pid, Errno> {
    let status = status.map_or(ptr::null_mut(), |status| &mut status.0);
    // SAFETY: Status is null or exclusively writable; C range-checks the PID.
    pid_result(unsafe { lunacy_waitpid(pid, status, options.0) })
}

fn pid_result(result: Pid) -> Result<Pid, Errno> {
    if result == -1 {
        Err(Errno::last())
    } else {
        Ok(result)
    }
}
