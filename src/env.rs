//! Environment access with either an owned copy or an unsafe borrowed value.
//!
//! No global locks are added. Unsafe mutation must coordinate with all process
//! environment access, including readers in C and other libraries.

use alloc::{ffi::CString, vec::Vec};
use core::ffi::CStr;

use crate::{Errno, sys};

/// Returns an owned copy of an environment value, without UTF-8 conversion.
///
/// `None` means absent (or an empty name / name containing '='). A present empty
/// value returns an empty CString. The copy remains valid across later lookups
/// and environment changes. Allocation failure follows alloc's usual behavior.
/// Use [`getenv_borrowed`] for unsafe access without copying.
///
/// The C helper reads environ directly, avoiding libc's potentially shared
/// getenv return buffer. Concurrent readers do not modify this storage. Unsafe
/// environment mutation must exclude readers, including this function, as
/// required by [`setenv`] and [`unsetenv`]. No global locking is introduced.
pub fn getenv(name: &CStr) -> Option<CString> {
    let mut bytes = Vec::<u8>::new();
    loop {
        // SAFETY: name is NUL-terminated. The helper only reads the environment
        // and writes at most capacity bytes. All environment mutation must
        // exclude concurrent readers. No borrowed environment pointer escapes.
        let needed =
            unsafe { sys::lunacy_getenv_copy(name.as_ptr(), bytes.as_mut_ptr(), bytes.capacity()) };
        if needed == 0 {
            return None;
        }
        if needed > bytes.capacity() {
            // No environment references survive this allocation. A custom Rust
            // allocator may reenter environment APIs, so look up the value again
            // afterward rather than retaining its old pointer or length.
            bytes.reserve_exact(needed);
            continue;
        }
        // SAFETY: The helper initialized exactly needed bytes, with one terminal
        // NUL and no interior NULs. These bytes now belong solely to the Vec.
        return Some(unsafe {
            bytes.set_len(needed);
            CString::from_vec_with_nul_unchecked(bytes)
        });
    }
}

/// Calls libc getenv once, borrowing its value without copying or decoding it.
///
/// `None` means the name is absent; `Some(c"")` is a present, empty value.
/// Absence is not an errno error. The result borrows libc's storage, not `name`.
/// Use [`getenv`] for a safe, independent copy.
///
/// # Safety
/// No environment mutation may race with this call. Follow the target libc's
/// thread-safety requirements for getenv itself as well.
///
/// For the entire chosen lifetime `'env`, the returned string must remain valid
/// and immutable. Prevent calls that may invalidate it, including setenv,
/// unsetenv, putenv, direct environ modification, and (on some libcs) another
/// getenv. This includes accesses by other threads or foreign libraries. The
/// caller must not infer a static lifetime merely because libc owns the storage.
pub unsafe fn getenv_borrowed<'env>(name: &CStr) -> Option<&'env CStr> {
    // SAFETY: name is NUL-terminated; caller guarantees environment access safety.
    let value = unsafe { sys::lunacy_getenv(name.as_ptr()) };
    if value.is_null() {
        None
    } else {
        // SAFETY: getenv returns a NUL-terminated value, and the caller guarantees
        // its validity and immutability for the chosen lifetime.
        Some(unsafe { CStr::from_ptr(value) })
    }
}

/// Calls libc setenv once. If overwrite is false, an existing value is retained.
///
/// libc copies the name and value; neither input must remain alive after return.
/// Empty names and names containing '=' produce the native EINVAL error.
///
/// # Safety
/// The caller must exclude concurrent environment readers and writers, including
/// accesses inside libc and foreign libraries, for this call. No borrowed getenv
/// value that this operation could invalidate may remain in use. The inputs
/// must also remain valid through the call; do not pass potentially invalidated
/// pointers into libc's own environment storage.
pub unsafe fn setenv(name: &CStr, value: &CStr, overwrite: bool) -> Result<(), Errno> {
    // SAFETY: Inputs are NUL-terminated, and the caller coordinates global access.
    sys::cvt(unsafe { sys::lunacy_setenv(name.as_ptr(), value.as_ptr(), overwrite.into()) })
        .map(|_| ())
}

/// Calls libc unsetenv once. Removing an absent name succeeds.
///
/// # Safety
/// The caller must exclude concurrent environment readers and writers, including
/// accesses inside libc and foreign libraries, for this call. No borrowed getenv
/// value that this operation could invalidate may remain in use. `name` must
/// remain valid through the call and must not alias storage invalidated by it.
pub unsafe fn unsetenv(name: &CStr) -> Result<(), Errno> {
    // SAFETY: name is NUL-terminated, and the caller coordinates global access.
    sys::cvt(unsafe { sys::lunacy_unsetenv(name.as_ptr()) }).map(|_| ())
}
