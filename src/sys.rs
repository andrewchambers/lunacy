use core::ffi::{c_int, c_void};

// Mirrored by abi.h; C static assertions check the native types against this.
#[repr(C, align(16))]
#[derive(Clone)]
pub(crate) struct Storage(pub [u8; 128]);

#[link(name = "c")]
unsafe extern "C" {
    pub fn lunacy_errno() -> c_int;
    pub fn lunacy_write(fd: c_int, buffer: *const u8, len: usize) -> isize;
    pub fn lunacy_print(buffer: *const u8, len: usize, stderr: c_int) -> isize;
    pub fn lunacy_read(fd: c_int, buffer: *mut u8, len: usize) -> isize;
    pub fn lunacy_pipe(fds: *mut c_int) -> c_int;
    pub fn lunacy_dup(fd: c_int) -> c_int;
    pub fn lunacy_dup2(source: c_int, target: c_int) -> c_int;
    pub fn lunacy_close(fd: c_int) -> c_int;
    pub fn lunacy_getenv(name: *const core::ffi::c_char) -> *const core::ffi::c_char;
    pub fn lunacy_getenv_copy(
        name: *const core::ffi::c_char,
        buffer: *mut u8,
        capacity: usize,
    ) -> usize;
    pub fn lunacy_setenv(
        name: *const core::ffi::c_char,
        value: *const core::ffi::c_char,
        overwrite: c_int,
    ) -> c_int;
    pub fn lunacy_unsetenv(name: *const core::ffi::c_char) -> c_int;
    pub fn lunacy_alloc(size: usize, align: usize) -> *mut c_void;
    pub fn lunacy_alloc_zeroed(size: usize, align: usize) -> *mut c_void;
    pub fn lunacy_realloc(
        ptr: *mut c_void,
        old_size: usize,
        align: usize,
        new_size: usize,
    ) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn abort() -> !;

    pub fn lunacy_socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    pub fn lunacy_socketpair(domain: c_int, kind: c_int, protocol: c_int, fds: *mut c_int)
    -> c_int;
    pub fn lunacy_bind(fd: c_int, addr: *const Storage, len: usize) -> c_int;
    pub fn lunacy_connect(fd: c_int, addr: *const Storage, len: usize) -> c_int;
    pub fn lunacy_listen(fd: c_int, backlog: c_int) -> c_int;
    pub fn lunacy_accept(fd: c_int, addr: *mut Storage, len: *mut usize) -> c_int;
    pub fn lunacy_getsockname(fd: c_int, addr: *mut Storage, len: *mut usize) -> c_int;
    pub fn lunacy_getpeername(fd: c_int, addr: *mut Storage, len: *mut usize) -> c_int;
    pub fn lunacy_shutdown(fd: c_int, how: c_int) -> c_int;
    pub fn lunacy_send(fd: c_int, buf: *const u8, len: usize, flags: c_int) -> isize;
    pub fn lunacy_recv(fd: c_int, buf: *mut u8, len: usize, flags: c_int) -> isize;
    pub fn lunacy_addr_v4(out: *mut Storage, ip: *const u8, port: u16) -> usize;
    pub fn lunacy_addr_v6(
        out: *mut Storage,
        ip: *const u8,
        port: u16,
        flowinfo: u32,
        scope_id: u32,
    ) -> usize;
    pub fn lunacy_addr_unix(
        out: *mut Storage,
        path: *const core::ffi::c_char,
        len: usize,
        size: *mut usize,
    ) -> c_int;
    pub fn lunacy_addr_family(addr: *const Storage) -> c_int;
    pub fn lunacy_addr_get_v4(
        addr: *const Storage,
        len: usize,
        ip: *mut u8,
        port: *mut u16,
    ) -> c_int;
    pub fn lunacy_addr_get_v6(
        addr: *const Storage,
        len: usize,
        ip: *mut u8,
        port: *mut u16,
        flowinfo: *mut u32,
        scope_id: *mut u32,
    ) -> c_int;

    pub fn lunacy_poll(fds: *mut c_void, count: usize, timeout: c_int) -> c_int;
    pub fn lunacy_fd_setsize() -> c_int;
    pub fn lunacy_fd_zero(set: *mut Storage);
    pub fn lunacy_fd_set(set: *mut Storage, fd: c_int) -> c_int;
    pub fn lunacy_fd_clr(set: *mut Storage, fd: c_int) -> c_int;
    pub fn lunacy_fd_isset(set: *const Storage, fd: c_int) -> c_int;
    pub fn lunacy_select(
        nfds: c_int,
        readfds: *mut Storage,
        writefds: *mut Storage,
        exceptfds: *mut Storage,
        seconds: *mut i64,
        microseconds: *mut i64,
    ) -> c_int;

    pub fn lunacy_clock_gettime(clock: i64, seconds: *mut i64, nanoseconds: *mut i64) -> c_int;
    pub fn lunacy_clock_getres(clock: i64, seconds: *mut i64, nanoseconds: *mut i64) -> c_int;
    pub fn lunacy_nanosleep(
        seconds: i64,
        nanoseconds: i64,
        remaining_seconds: *mut i64,
        remaining_nanoseconds: *mut i64,
    ) -> c_int;
}

#[cfg(feature = "pthread")]
unsafe extern "C" {
    pub fn lunacy_thread_create(
        out: *mut Storage,
        entry: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
        context: *mut c_void,
        set_stack_size: c_int,
        stack_size: usize,
    ) -> c_int;
    pub fn lunacy_thread_join(thread: *const Storage) -> c_int;
    pub fn lunacy_thread_detach(thread: *const Storage) -> c_int;
    pub fn lunacy_thread_is_current(thread: *const Storage) -> c_int;
    pub fn lunacy_thread_yield() -> c_int;
    pub fn lunacy_mutex_create(out: *mut *mut c_void) -> c_int;
    pub fn lunacy_mutex_lock(mutex: *mut c_void) -> c_int;
    pub fn lunacy_mutex_trylock(mutex: *mut c_void) -> c_int;
    pub fn lunacy_mutex_unlock(mutex: *mut c_void) -> c_int;
    pub fn lunacy_mutex_destroy(mutex: *mut c_void) -> c_int;
}

#[cfg(feature = "pthread")]
pub(crate) fn cvt_pthread(result: c_int) -> Result<(), crate::Errno> {
    if result == 0 {
        Ok(())
    } else {
        Err(crate::Errno::from_raw(result))
    }
}

pub(crate) fn cvt(result: c_int) -> Result<c_int, crate::Errno> {
    if result == -1 {
        Err(crate::Errno::last())
    } else {
        Ok(result)
    }
}

pub(crate) fn cvt_size(result: isize) -> Result<usize, crate::Errno> {
    if result == -1 {
        Err(crate::Errno::last())
    } else {
        Ok(result as usize)
    }
}
