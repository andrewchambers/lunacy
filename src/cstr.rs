//! Allocation-backed C string helpers.

use alloc::ffi::{CString, NulError};
use alloc::vec::Vec;

/// Builds an owned C string from bytes.
///
/// This is a small `alloc`-using convenience around [`CString::new`].
pub fn cstring(bytes: impl Into<Vec<u8>>) -> core::result::Result<CString, NulError> {
    CString::new(bytes)
}

#[cfg(test)]
mod tests {
    use super::cstring;

    #[test]
    fn rejects_interior_nul() {
        assert!(cstring(b"a\0b".to_vec()).is_err());
    }
}
