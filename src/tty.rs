//! Terminal control wrappers.
//!
//! Native `termios` and `winsize` layout stays in the C shim. Rust owns an
//! opaque raw-mode guard and neutral window-size values.

use core::ptr::NonNull;

use crate::{
    Errno, Result,
    fd::{FdArg, RawFd},
};

/// Terminal window size in character cells.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WindowSize {
    rows: u16,
    columns: u16,
}

impl WindowSize {
    /// Creates a window size from row and column counts.
    pub const fn new(rows: u16, columns: u16) -> Self {
        Self { rows, columns }
    }

    /// Returns the number of rows.
    pub const fn rows(self) -> u16 {
        self.rows
    }

    /// Returns the number of columns.
    pub const fn columns(self) -> u16 {
        self.columns
    }
}

/// A guard that restores a terminal's original mode when dropped.
pub struct RawMode {
    raw: Option<NonNull<crate::ffi::c_void>>,
}

impl RawMode {
    /// Enables linenoise-style raw mode for `fd`.
    ///
    /// The original terminal mode is restored when the guard is dropped.
    pub fn enable(fd: impl FdArg) -> Result<Self> {
        let raw =
            NonNull::new(unsafe { lunacy_raw_mode_enable(fd.raw_fd()) }).ok_or_else(Errno::last)?;

        Ok(Self { raw: Some(raw) })
    }

    /// Restores the terminal mode before this guard would otherwise be dropped.
    ///
    /// If restoration fails, the guard keeps the saved mode and `Drop` will
    /// retry restoration later.
    pub fn restore(&mut self) -> Result<()> {
        let Some(raw) = self.raw else {
            return Ok(());
        };

        cvt_int(unsafe { lunacy_raw_mode_restore(raw.as_ptr()) })?;
        unsafe {
            lunacy_raw_mode_free(raw.as_ptr());
        }
        self.raw = None;
        Ok(())
    }
}

impl Drop for RawMode {
    fn drop(&mut self) {
        if let Some(raw) = self.raw.take() {
            unsafe {
                let _ = lunacy_raw_mode_restore(raw.as_ptr());
                lunacy_raw_mode_free(raw.as_ptr());
            }
        }
    }
}

/// Returns whether `fd` refers to a terminal.
pub fn isatty(fd: impl FdArg) -> Result<bool> {
    cvt_bool(unsafe { lunacy_tty_isatty(fd.raw_fd()) })
}

/// Returns the terminal window size for `fd`.
pub fn window_size(fd: impl FdArg) -> Result<WindowSize> {
    let mut rows = 0;
    let mut columns = 0;
    cvt_int(unsafe { lunacy_tty_window_size(fd.raw_fd(), &mut rows, &mut columns) })?;

    Ok(WindowSize::new(rows, columns))
}

fn cvt_int(value: crate::ffi::c_int) -> Result<crate::ffi::c_int> {
    if value < 0 {
        Err(Errno::last())
    } else {
        Ok(value)
    }
}

fn cvt_bool(value: crate::ffi::c_int) -> Result<bool> {
    cvt_int(value).map(|value| value != 0)
}

unsafe extern "C" {
    fn lunacy_tty_isatty(fd: RawFd) -> crate::ffi::c_int;
    fn lunacy_raw_mode_enable(fd: RawFd) -> *mut crate::ffi::c_void;
    fn lunacy_raw_mode_restore(raw: *mut crate::ffi::c_void) -> crate::ffi::c_int;
    fn lunacy_raw_mode_free(raw: *mut crate::ffi::c_void);
    fn lunacy_tty_window_size(fd: RawFd, rows: *mut u16, columns: *mut u16) -> crate::ffi::c_int;
}

#[cfg(test)]
mod tests {
    use super::{RawMode, isatty, window_size};
    use crate::fd;

    #[test]
    fn pipe_is_not_a_tty() {
        let (read_fd, write_fd) = fd::pipe().unwrap();

        assert!(!isatty(&read_fd).unwrap());

        fd::close(read_fd).unwrap();
        fd::close(write_fd).unwrap();
    }

    #[test]
    fn window_size_reports_error_for_pipe() {
        let (read_fd, write_fd) = fd::pipe().unwrap();

        assert!(window_size(&read_fd).is_err());

        fd::close(read_fd).unwrap();
        fd::close(write_fd).unwrap();
    }

    #[test]
    fn raw_mode_reports_error_for_pipe() {
        let (read_fd, write_fd) = fd::pipe().unwrap();

        assert!(RawMode::enable(&read_fd).is_err());

        fd::close(read_fd).unwrap();
        fd::close(write_fd).unwrap();
    }
}
