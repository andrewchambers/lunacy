//! Threads and mutexes backed by pthreads, enabled by the `pthread` feature.
//!
//! Thread panics abort at the C callback boundary, even in std applications with
//! unwinding enabled. Foreign cancellation, pthread_exit through Rust frames,
//! and manipulation of these pthreads outside the wrapper are unsupported and
//! must not be performed by unsafe/foreign code while Rust invariants are live.

use alloc::{boxed::Box, sync::Arc};
use core::{
    cell::UnsafeCell,
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{Errno, sys};

pub use crate::sync::{Mutex, MutexGuard};

struct Packet<T> {
    result: UnsafeCell<Option<T>>,
    finished: AtomicBool,
}

// SAFETY: Only the worker writes result. The handle reads it only after a
// successful pthread_join, which synchronizes with termination. Arc keeps the
// packet alive through both owners; the last owner alone destroys a leftover
// result. T: Send permits that destruction on either thread. No &T is shared.
unsafe impl<T: Send> Sync for Packet<T> {}

struct Start<F, T> {
    function: F,
    packet: Arc<Packet<T>>,
}

unsafe extern "C" fn trampoline<F, T>(context: *mut c_void) -> *mut c_void
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    // SAFETY: Successful create transfers the unique Box<Start<F, T>> to us.
    let Start { function, packet } = unsafe { *Box::from_raw(context.cast::<Start<F, T>>()) };
    let value = function();
    // SAFETY: This worker is the only writer. The handle cannot read until join.
    unsafe {
        *packet.result.get() = Some(value);
    }
    packet.finished.store(true, Ordering::Release);
    // On detach, the last Arc also destroys an unclaimed return value here.
    drop(packet);
    ptr::null_mut()
}

/// A uniquely owned joinable pthread. Dropping it detaches without waiting.
///
/// Detaching lets the worker finish normally; captured state and unclaimed
/// return values are still dropped. As with pthreads, returning from the process's
/// main function terminates every remaining thread. Join to ensure completion.
#[must_use = "dropping a join handle detaches the thread"]
pub struct JoinHandle<T> {
    native: sys::Storage,
    packet: Arc<Packet<T>>,
    joinable: bool,
}

impl<T> JoinHandle<T> {
    /// Waits for thread termination, then transfers its return value to the caller.
    ///
    /// This consumes the handle. Native errors are returned directly; a failed
    /// join is followed by the handle's normal detach-on-drop cleanup. Joining
    /// the current thread returns EDEADLK. Panics are process-aborting, not values
    /// returned by join.
    pub fn join(mut self) -> Result<T, Errno> {
        // SAFETY: This handle exclusively owns a valid joinable pthread ID.
        sys::cvt_pthread(unsafe { sys::lunacy_thread_join(&self.native) })?;
        self.joinable = false;
        // SAFETY: Successful join guarantees that the only writer has terminated.
        // The other Arc was dropped by the trampoline, and no other reader exists.
        match unsafe { (*self.packet.result.get()).take() } {
            Some(value) => Ok(value),
            // A normally returning trampoline always initializes the packet.
            None => crate::abort(),
        }
    }

    /// Calls pthread_detach once, relinquishing the right to join without waiting.
    ///
    /// Native errors are reported; a failed explicit detach is not retried and
    /// may leave native resources allocated. The Rust worker/result storage is
    /// independently reclaimed when its last owner disappears.
    pub fn detach(mut self) -> Result<(), Errno> {
        self.joinable = false;
        // SAFETY: This handle exclusively owns the still-joinable pthread ID.
        sys::cvt_pthread(unsafe { sys::lunacy_thread_detach(&self.native) })
    }

    /// Whether the closure has returned and stored its result.
    ///
    /// Thread-local destructors and other pthread cleanup may still be running;
    /// only join guarantees that the entire thread has terminated.
    pub fn is_finished(&self) -> bool {
        self.packet.finished.load(Ordering::Acquire)
    }

    /// Whether this live join handle refers to the calling thread.
    pub fn is_current(&self) -> bool {
        // SAFETY: A joinable thread's ID remains valid until join/detach, and the
        // shared borrow prevents either operation while we compare it.
        unsafe { sys::lunacy_thread_is_current(&self.native) != 0 }
    }
}

impl<T> Drop for JoinHandle<T> {
    fn drop(&mut self) {
        if self.joinable {
            // SAFETY: This is the final use of our uniquely owned pthread ID.
            // Ignore native cleanup errors, as for OwnedFd's close-on-drop.
            unsafe {
                sys::lunacy_thread_detach(&self.native);
            }
        }
    }
}

/// Creates a pthread with default attributes, running a Rust closure once.
///
/// The closure and result must be Send + 'static because the thread can outlive
/// its creator. Two Rust allocations hold the closure and shared result state;
/// allocation failure follows alloc's usual behavior. Creation errors return
/// native error codes and drop the unstarted closure. Panics in the worker abort.
///
/// ```compile_fail
/// let local = String::from("borrowed");
/// lunacy::pthread::spawn(|| local.len()); // Closure can outlive local.
/// ```
///
/// ```compile_fail
/// let local = std::rc::Rc::new(1);
/// lunacy::pthread::spawn(move || *local); // Rc cannot cross threads.
/// ```
pub fn spawn<F, T>(function: F) -> Result<JoinHandle<T>, Errno>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    spawn_impl(function, None)
}

/// Creates a pthread after setting its stack size in bytes with
/// pthread_attr_setstacksize. libc validates the size and retains its default
/// guard-size policy. A too-small or otherwise unsupported size returns EINVAL.
pub fn spawn_with_stack_size<F, T>(stack_size: usize, function: F) -> Result<JoinHandle<T>, Errno>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    spawn_impl(function, Some(stack_size))
}

fn spawn_impl<F, T>(function: F, stack_size: Option<usize>) -> Result<JoinHandle<T>, Errno>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    let packet = Arc::new(Packet {
        result: UnsafeCell::new(None),
        finished: AtomicBool::new(false),
    });
    let context = Box::into_raw(Box::new(Start {
        function,
        packet: Arc::clone(&packet),
    }));
    let mut native = sys::Storage([0; 128]);
    // SAFETY: The callback matches the boxed context's exact types. On success
    // the worker owns it; on failure no worker started and we reclaim it below.
    let error = unsafe {
        sys::lunacy_thread_create(
            &mut native,
            trampoline::<F, T>,
            context.cast(),
            stack_size.is_some().into(),
            stack_size.unwrap_or(0),
        )
    };
    if error != 0 {
        // SAFETY: pthread_create failed, so ownership never reached the callback.
        drop(unsafe { Box::from_raw(context) });
        return Err(Errno::from_raw(error));
    }
    Ok(JoinHandle {
        native,
        packet,
        joinable: true,
    })
}

/// Calls sched_yield once, yielding according to the current scheduling policy.
/// It does not promise that another thread will run.
pub fn yield_now() -> Result<(), Errno> {
    // SAFETY: sched_yield has no pointer arguments or preconditions.
    sys::cvt(unsafe { sys::lunacy_thread_yield() }).map(|_| ())
}
