//! Optional pthread wrappers.
//!
//! This module is available with the `pthread` feature. It keeps pthread
//! storage opaque to Rust: `pthread_t` and `pthread_mutex_t` are allocated and
//! initialized by the C shim.

use alloc::boxed::Box;
use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

use crate::{Errno, Result};

/// A joinable pthread.
///
/// Dropping a `Thread` detaches it. Use [`join`](Self::join) to wait for it.
pub struct Thread {
    raw: Option<NonNull<crate::ffi::c_void>>,
}

unsafe impl Send for Thread {}

impl Thread {
    /// Spawns a pthread that runs `f`.
    pub fn spawn<F>(f: F) -> Result<Self>
    where
        F: FnOnce() + Send + 'static,
    {
        spawn(f)
    }

    /// Waits for the thread to finish.
    ///
    /// The handle is consumed even if `pthread_join` reports an error.
    pub fn join(mut self) -> Result<()> {
        let raw = self.take_raw();
        cvt_int(unsafe { lunacy_pthread_join(raw.as_ptr()) })
    }

    /// Detaches the thread.
    ///
    /// The handle is consumed even if `pthread_detach` reports an error.
    pub fn detach(mut self) -> Result<()> {
        let raw = self.take_raw();
        cvt_int(unsafe { lunacy_pthread_detach(raw.as_ptr()) })
    }

    fn take_raw(&mut self) -> NonNull<crate::ffi::c_void> {
        self.raw
            .take()
            .expect("thread handle must only be consumed once")
    }
}

impl Drop for Thread {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            let _ = unsafe { lunacy_pthread_detach(raw.as_ptr()) };
        }
    }
}

/// Spawns a pthread that runs `f`.
pub fn spawn<F>(f: F) -> Result<Thread>
where
    F: FnOnce() + Send + 'static,
{
    let boxed = Box::new(f);
    let arg = Box::into_raw(boxed).cast::<crate::ffi::c_void>();
    let raw = unsafe { lunacy_pthread_create(thread_main::<F>, arg) };

    match NonNull::new(raw) {
        Some(raw) => Ok(Thread { raw: Some(raw) }),
        None => {
            unsafe {
                drop(Box::from_raw(arg.cast::<F>()));
            }
            Err(Errno::last())
        }
    }
}

extern "C" fn thread_main<F>(arg: *mut crate::ffi::c_void) -> *mut crate::ffi::c_void
where
    F: FnOnce(),
{
    let f = unsafe { Box::from_raw(arg.cast::<F>()) };
    f();
    core::ptr::null_mut()
}

/// A pthread-backed mutual exclusion primitive.
pub struct Mutex<T> {
    raw: NonNull<crate::ffi::c_void>,
    value: UnsafeCell<T>,
}

unsafe impl<T: Send> Send for Mutex<T> {}
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Creates a mutex containing `value`.
    pub fn new(value: T) -> Result<Self> {
        let raw = NonNull::new(unsafe { lunacy_pthread_mutex_create() }).ok_or_else(Errno::last)?;

        Ok(Self {
            raw,
            value: UnsafeCell::new(value),
        })
    }

    /// Acquires the mutex.
    pub fn lock(&self) -> Result<MutexGuard<'_, T>> {
        cvt_int(unsafe { lunacy_pthread_mutex_lock(self.raw.as_ptr()) })?;

        Ok(MutexGuard {
            mutex: self,
            _not_send: PhantomData,
        })
    }

    /// Returns a mutable reference to the contained value.
    ///
    /// This does not lock because a mutable borrow of the mutex already
    /// excludes other Rust access.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }
}

impl<T> Drop for Mutex<T> {
    fn drop(&mut self) {
        let _ = unsafe { lunacy_pthread_mutex_destroy(self.raw.as_ptr()) };
    }
}

/// A held [`Mutex`] lock.
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    _not_send: PhantomData<*mut ()>,
}

impl<T> MutexGuard<'_, T> {
    /// Unlocks the mutex before the guard would otherwise be dropped.
    pub fn unlock(self) -> Result<()> {
        let raw = self.mutex.raw;
        core::mem::forget(self);
        cvt_int(unsafe { lunacy_pthread_mutex_unlock(raw.as_ptr()) })
    }
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        let _ = unsafe { lunacy_pthread_mutex_unlock(self.mutex.raw.as_ptr()) };
    }
}

fn cvt_int(value: crate::ffi::c_int) -> Result<()> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(())
    }
}

unsafe extern "C" {
    fn lunacy_pthread_create(
        start: extern "C" fn(*mut crate::ffi::c_void) -> *mut crate::ffi::c_void,
        arg: *mut crate::ffi::c_void,
    ) -> *mut crate::ffi::c_void;
    fn lunacy_pthread_detach(thread: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_pthread_join(thread: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_pthread_mutex_create() -> *mut crate::ffi::c_void;
    fn lunacy_pthread_mutex_lock(mutex: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_pthread_mutex_unlock(mutex: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_pthread_mutex_destroy(mutex: *mut crate::ffi::c_void) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;

    use super::{Mutex, Thread};

    #[test]
    fn locks_and_unlocks_mutex() {
        let mutex = Mutex::new(1).unwrap();

        {
            let mut value = mutex.lock().unwrap();
            *value += 1;
        }

        assert_eq!(*mutex.lock().unwrap(), 2);
    }

    #[test]
    fn spawned_thread_can_update_shared_state() {
        let value = Arc::new(Mutex::new(0).unwrap());
        let thread_value = Arc::clone(&value);

        let thread = Thread::spawn(move || {
            *thread_value.lock().unwrap() = 7;
        })
        .unwrap();

        thread.join().unwrap();

        assert_eq!(*value.lock().unwrap(), 7);
    }
}
