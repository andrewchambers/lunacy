//! pthread-backed synchronization protecting Rust data.

use core::{
    cell::UnsafeCell,
    ffi::c_void,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Deref, DerefMut},
    ptr::{self, NonNull},
};

use crate::{Errno, sys};

/// A non-recursive pthread mutex protecting a Rust value.
///
/// Uses PTHREAD_MUTEX_ERRORCHECK: recursive lock returns EDEADLK, and try_lock on
/// a locked mutex returns EBUSY. There is no poisoning; catching a Rust panic
/// while a guard is held leaves subsequent callers responsible for the data's
/// logical consistency. Thread callback panics themselves abort the process.
///
/// The native mutex has its own libc allocation so it never moves after init.
/// The Rust wrapper can be moved while unborrowed. Forgetting a guard leaves the
/// mutex locked; dropping that mutex then deliberately leaks the native allocation
/// rather than attempting to destroy a locked pthread mutex.
pub struct Mutex<T> {
    native: NonNull<c_void>,
    value: UnsafeCell<T>,
}

// SAFETY: Moving the wrapper transfers T; the native allocation stays fixed.
unsafe impl<T: Send> Send for Mutex<T> {}
// SAFETY: All shared access to T is serialized by the pthread mutex. T need only
// be Send, because references to it are not shared between simultaneous lockers.
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    /// Allocates and initializes the native mutex. Native errors, including
    /// ENOMEM, are returned and the supplied value is dropped on failure.
    pub fn new(value: T) -> Result<Self, Errno> {
        let mut native = ptr::null_mut();
        // SAFETY: C writes an initialized, stable mutex pointer on success only.
        sys::cvt_pthread(unsafe { sys::lunacy_mutex_create(&mut native) })?;
        // SAFETY: Successful native initialization guarantees a non-null pointer.
        Ok(Self {
            native: unsafe { NonNull::new_unchecked(native) },
            value: UnsafeCell::new(value),
        })
    }

    /// Calls pthread_mutex_lock, blocking until exclusive access is acquired.
    /// A recursive call on the locking thread returns EDEADLK.
    pub fn lock(&self) -> Result<MutexGuard<'_, T>, Errno> {
        // SAFETY: The native mutex is initialized and remains live for this borrow.
        sys::cvt_pthread(unsafe { sys::lunacy_mutex_lock(self.native.as_ptr()) })?;
        Ok(MutexGuard {
            mutex: self,
            _not_send: PhantomData,
        })
    }

    /// Calls pthread_mutex_trylock once. EBUSY means another guard holds the lock.
    pub fn try_lock(&self) -> Result<MutexGuard<'_, T>, Errno> {
        // SAFETY: The native mutex is initialized and remains live for this borrow.
        sys::cvt_pthread(unsafe { sys::lunacy_mutex_trylock(self.native.as_ptr()) })?;
        Ok(MutexGuard {
            mutex: self,
            _not_send: PhantomData,
        })
    }

    /// Accesses the data without locking when Rust guarantees exclusive ownership.
    pub fn get_mut(&mut self) -> &mut T {
        self.value.get_mut()
    }

    /// Consumes the mutex and returns its value without locking.
    pub fn into_inner(self) -> T {
        let this = ManuallyDrop::new(self);
        // SAFETY: Exclusive ownership precludes live guards/waiters. C detects
        // a forgotten guard and leaks rather than destroying a locked mutex.
        unsafe {
            sys::lunacy_mutex_destroy(this.native.as_ptr());
        }
        // SAFETY: No other references remain. ManuallyDrop prevents a second drop.
        unsafe { this.value.get().read() }
    }
}

impl<T> Drop for Mutex<T> {
    fn drop(&mut self) {
        // SAFETY: The wrapper is exclusively owned. The C helper only destroys
        // unlocked mutexes and leaves forgotten-lock allocations intact.
        unsafe {
            sys::lunacy_mutex_destroy(self.native.as_ptr());
        }
    }
}

/// Exclusive access to mutex data. Dropping this guard unlocks on the same thread.
///
/// ```compile_fail
/// let mutex = Box::leak(Box::new(lunacy::pthread::Mutex::new(0).unwrap()));
/// let guard = mutex.lock().unwrap();
/// std::thread::spawn(move || drop(guard)); // pthread unlock must stay on its owner.
/// ```
///
/// ```compile_fail
/// let mutex = lunacy::pthread::Mutex::new(std::cell::Cell::new(0)).unwrap();
/// let guard = mutex.lock().unwrap();
/// std::thread::scope(|scope| {
///     scope.spawn(|| guard.set(1)); // Sharing a guard requires Sync data.
/// });
/// ```
pub struct MutexGuard<'a, T> {
    mutex: &'a Mutex<T>,
    _not_send: PhantomData<*mut ()>,
}

// SAFETY: Sharing a reference to the guard only permits shared access to T. The
// guard itself cannot move threads, and borrowing it prevents its owner dropping it.
unsafe impl<T: Send + Sync> Sync for MutexGuard<'_, T> {}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: This guard uniquely holds the mutex until it is dropped.
        unsafe { &*self.mutex.value.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: The lock excludes other lockers and &mut self excludes aliases.
        unsafe { &mut *self.mutex.value.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        // SAFETY: This guard owns the lock and cannot have moved to another thread.
        // Failure would violate an internal invariant; do not continue silently.
        if unsafe { sys::lunacy_mutex_unlock(self.mutex.native.as_ptr()) } != 0 {
            crate::abort();
        }
    }
}
