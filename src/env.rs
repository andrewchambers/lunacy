//! Process environment wrappers.

use alloc::{borrow::ToOwned, ffi::CString};
use core::ffi::CStr;

use crate::{Errno, Result};

/// Returns an owned copy of an environment variable's value.
///
/// This returns `None` when `name` is not present in the process environment.
/// The value is copied so later environment changes do not invalidate the
/// returned string.
pub fn getenv(name: &CStr) -> Option<CString> {
    let value = unsafe { crate::ffi::getenv(name.as_ptr()) };

    if value.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(value) }.to_owned())
    }
}

/// Sets an environment variable using libc `setenv`.
///
/// When `overwrite` is false, an existing variable is left unchanged.
///
/// # Safety
///
/// Modifying the process environment is globally visible and can race with
/// environment access in other threads or foreign code. The caller must ensure
/// this call does not run concurrently with any environment reads or writes.
pub unsafe fn setenv(name: &CStr, value: &CStr, overwrite: bool) -> Result<()> {
    let overwrite = if overwrite { 1 } else { 0 };
    let status = unsafe { crate::ffi::setenv(name.as_ptr(), value.as_ptr(), overwrite) };

    if status < 0 {
        Err(Errno::last())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloc::{ffi::CString, format};

    use super::{getenv, setenv};

    #[test]
    fn reads_and_sets_environment_variables() {
        let name = CString::new(format!("LUNACY_TEST_ENV_{}", std::process::id())).unwrap();
        let first = CString::new("first").unwrap();
        let second = CString::new("second").unwrap();

        assert!(getenv(&name).is_none());

        unsafe {
            setenv(&name, &first, true).unwrap();
        }
        assert_eq!(getenv(&name).unwrap().as_bytes(), b"first");

        unsafe {
            setenv(&name, &second, false).unwrap();
        }
        assert_eq!(getenv(&name).unwrap().as_bytes(), b"first");

        unsafe {
            setenv(&name, &second, true).unwrap();
        }
        assert_eq!(getenv(&name).unwrap().as_bytes(), b"second");
    }
}
