// Harness-free: mutation runs before any threads are started. Threads are used
// only for the final, read-only test, without sibling tests mutating the process.
use lunacy::{
    Errno,
    env::{getenv, getenv_borrowed, setenv, unsetenv},
};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    ffi::CString,
    sync::atomic::{AtomicU8, Ordering},
};

const REENTRANT_NAME: &std::ffi::CStr = c"LUNACY_ENV_ALLOCATOR_TEST";
const GROWN_VALUE: &std::ffi::CStr = c"a replacement longer than the initial allocation";
static ON_ALLOC: AtomicU8 = AtomicU8::new(0);

struct ReentrantAllocator;

// SAFETY: Storage operations delegate to System. The callback is armed only by
// the single-threaded test below, at a point with no borrowed environment values.
unsafe impl GlobalAlloc for ReentrantAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let action = ON_ALLOC.swap(0, Ordering::Relaxed);
        // SAFETY: The callback runs before threads are started. getenv has
        // finished its C read and holds no environment pointers during allocation.
        let result = unsafe {
            match action {
                1 => setenv(REENTRANT_NAME, GROWN_VALUE, true),
                2 => unsetenv(REENTRANT_NAME),
                _ => Ok(()),
            }
        };
        if result.is_err() {
            std::process::abort();
        }
        // SAFETY: The caller's valid Layout is passed through unchanged.
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: This allocation originated in System with the same layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: ReentrantAllocator = ReentrantAllocator;

fn main() {
    let name = c"LUNACY_ENV_TEST_VALUE";
    // SAFETY: This dedicated test process creates no threads. All environment
    // accesses are sequential; every borrowed value is consumed by its assertion
    // before another environment call. Mutation inputs are separate storage.
    unsafe {
        unsetenv(name).unwrap();
        assert!(getenv_borrowed(name).is_none());
        unsetenv(name).unwrap(); // Removing an absent variable succeeds.

        setenv(name, c"first", false).unwrap();
        assert_eq!(getenv_borrowed(name), Some(c"first"));
        setenv(name, c"ignored", false).unwrap();
        assert_eq!(getenv_borrowed(name), Some(c"first"));
        setenv(name, c"second", true).unwrap();
        assert_eq!(getenv_borrowed(name), Some(c"second"));

        // setenv copies its inputs rather than retaining pointers to them.
        {
            let temporary = CString::new(vec![b'v', b'=', 0xff]).unwrap();
            setenv(name, &temporary, true).unwrap();
        }
        assert_eq!(
            getenv_borrowed(name).unwrap().to_bytes(),
            &[b'v', b'=', 0xff]
        );

        setenv(name, c"", true).unwrap();
        assert_eq!(getenv_borrowed(name), Some(c""));
        unsetenv(name).unwrap();
        assert!(getenv_borrowed(name).is_none());

        for invalid in [c"", c"INVALID=NAME"] {
            assert_eq!(setenv(invalid, c"value", true), Err(Errno::EINVAL));
            assert_eq!(unsetenv(invalid), Err(Errno::EINVAL));
        }
    }

    // SAFETY: Still single-threaded; these calls invalidate no borrowed results.
    unsafe {
        setenv(name, c"owned original", true).unwrap();
    }
    let original = getenv(name).unwrap();
    unsafe {
        setenv(name, c"owned replacement", true).unwrap();
    }
    let replacement = getenv(name).unwrap();
    unsafe {
        unsetenv(name).unwrap();
    }
    assert_eq!(original.as_c_str(), c"owned original");
    assert_eq!(replacement.as_c_str(), c"owned replacement");
    assert_eq!(getenv(name), None);
    assert_eq!(getenv(c""), None);
    assert_eq!(getenv(c"INVALID=NAME"), None);

    unsafe {
        setenv(name, c"", true).unwrap();
    }
    assert_eq!(getenv(name).as_deref(), Some(c""));
    let non_utf8 = CString::new(vec![0xff, b'=', b'x']).unwrap();
    unsafe {
        setenv(name, &non_utf8, true).unwrap();
    }
    assert_eq!(getenv(name).unwrap().to_bytes(), non_utf8.to_bytes());

    // A custom allocator can change the environment between sizing and copying.
    unsafe {
        setenv(REENTRANT_NAME, c"x", true).unwrap();
    }
    ON_ALLOC.store(1, Ordering::Relaxed);
    assert_eq!(getenv(REENTRANT_NAME).as_deref(), Some(GROWN_VALUE));
    ON_ALLOC.store(2, Ordering::Relaxed);
    assert_eq!(getenv(REENTRANT_NAME), None);
    assert_eq!(ON_ALLOC.load(Ordering::Relaxed), 0);

    // No mutation from this point onward. Concurrent owned lookups must not
    // share scratch storage or accidentally invalidate each other's values.
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                for _ in 0..100 {
                    let copy = getenv(name).unwrap();
                    assert_eq!(getenv(REENTRANT_NAME), None);
                    assert_eq!(copy.to_bytes(), &[0xff, b'=', b'x']);
                }
            });
        }
    });
    println!("environment checks passed");
}
