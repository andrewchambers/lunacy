//! Borrowed C command-line arguments, including `argv[0]`.

use core::{
    ffi::{CStr, c_char},
    marker::PhantomData,
};

/// A read-only view of argc/argv. No allocation or Unicode conversion is made.
///
/// `lunacy_main!(entry)` passes this view to `fn entry(args: Args<'_>) -> i32`.
/// Argument storage must not be modified by foreign code while borrowed.
pub struct Args<'a> {
    argc: usize,
    argv: *const *const c_char,
    _borrow: PhantomData<&'a CStr>,
}

impl<'a> Args<'a> {
    /// Borrows a C argument vector.
    ///
    /// # Safety
    /// `argv` must contain at least `argc` initialized pointers to valid,
    /// NUL-terminated strings. The pointers and strings must remain readable and
    /// unmodified for `'a`. For zero arguments, `argv` may be null.
    pub unsafe fn from_raw(argc: usize, argv: *const *const c_char) -> Self {
        Self {
            argc,
            argv,
            _borrow: PhantomData,
        }
    }

    /// Returns argc, including `argv[0]` when present.
    pub fn len(&self) -> usize {
        self.argc
    }

    /// Whether argc is zero.
    pub fn is_empty(&self) -> bool {
        self.argc == 0
    }

    /// Borrows `argv[index]`, or returns None when the index is outside argc.
    pub fn get(&self, index: usize) -> Option<&CStr> {
        if index >= self.argc {
            return None;
        }
        // SAFETY: The constructor guarantees a valid string at every in-range index.
        Some(unsafe { CStr::from_ptr(*self.argv.add(index)) })
    }
}
