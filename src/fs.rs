//! Unix filesystem wrappers.

use core::ffi::CStr;

use crate::{
    Result,
    fd::{FdArg, OwnedFd, RawFd},
};

pub use crate::ffi::Stat;

/// Opens a path with libc `open`.
pub fn open(path: &CStr, flags: crate::ffi::c_int) -> Result<OwnedFd> {
    cvt_fd(unsafe { crate::ffi::open(path.as_ptr(), flags) })
}

/// Opens a path with libc `open`, passing a mode for file creation.
pub fn open_mode(
    path: &CStr,
    flags: crate::ffi::c_int,
    mode: crate::ffi::mode_t,
) -> Result<OwnedFd> {
    cvt_fd(unsafe { crate::ffi::open(path.as_ptr(), flags, mode as crate::ffi::c_uint) })
}

/// Unlinks a filesystem path.
pub fn unlink(path: &CStr) -> Result<()> {
    cvt_int(unsafe { crate::ffi::unlink(path.as_ptr()) }).map(|_| ())
}

/// Creates a directory.
pub fn mkdir(path: &CStr, mode: crate::ffi::mode_t) -> Result<()> {
    cvt_int(unsafe { crate::ffi::mkdir(path.as_ptr(), mode) }).map(|_| ())
}

/// Removes a directory.
pub fn rmdir(path: &CStr) -> Result<()> {
    cvt_int(unsafe { crate::ffi::rmdir(path.as_ptr()) }).map(|_| ())
}

/// Renames a filesystem path.
pub fn rename(old: &CStr, new: &CStr) -> Result<()> {
    cvt_int(unsafe { crate::ffi::rename(old.as_ptr(), new.as_ptr()) }).map(|_| ())
}

/// Returns metadata for a path.
pub fn stat(path: &CStr) -> Result<Stat> {
    let mut stat = Stat::default();
    cvt_int(unsafe { lunacy_stat(path.as_ptr(), &mut stat) }).map(|_| stat)
}

/// Returns metadata for a file descriptor.
pub fn fstat(fd: impl FdArg) -> Result<Stat> {
    let mut stat = Stat::default();
    cvt_int(unsafe { lunacy_fstat(fd.raw_fd(), &mut stat) }).map(|_| stat)
}

fn cvt_fd(value: RawFd) -> Result<OwnedFd> {
    if value < 0 {
        Err(crate::Errno::last())
    } else {
        Ok(unsafe { OwnedFd::from_raw_fd(value) })
    }
}

fn cvt_int(value: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    if value < 0 {
        Err(crate::Errno::last())
    } else {
        Ok(value)
    }
}

unsafe extern "C" {
    fn lunacy_stat(path: *const crate::ffi::c_char, out: *mut Stat) -> crate::ffi::c_int;
    fn lunacy_fstat(fd: RawFd, out: *mut Stat) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use alloc::{ffi::CString, format};

    use super::{fstat, mkdir, rename, rmdir, stat, unlink};
    use crate::{fd, fs};

    fn tmp_path(name: &str) -> CString {
        CString::new(format!("/tmp/lunacy-{}-{name}", std::process::id())).unwrap()
    }

    #[test]
    fn creates_renames_stats_and_removes_directories() {
        let old = tmp_path("old-dir");
        let new = tmp_path("new-dir");

        let _ = rmdir(&old);
        let _ = rmdir(&new);

        mkdir(&old, 0o700).unwrap();
        let old_stat = stat(&old).unwrap();
        assert!(old_stat.mode != 0);

        rename(&old, &new).unwrap();
        let new_stat = stat(&new).unwrap();
        assert_eq!(old_stat.ino, new_stat.ino);

        rmdir(&new).unwrap();
    }

    #[test]
    fn stats_opened_files() {
        let path = tmp_path("file");
        std::fs::write(path.to_str().unwrap(), b"abc").unwrap();

        let file = fs::open(&path, 0).unwrap();
        let stat = fstat(&file).unwrap();
        assert_eq!(stat.size, 3);

        fd::close(file).unwrap();
        unlink(&path).unwrap();
    }
}
