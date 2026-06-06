//! Minimal no_std process entry support.

use core::{ffi::CStr, marker::PhantomData};

/// Process arguments passed by the C runtime.
#[derive(Clone, Copy, Debug)]
pub struct Args<'a> {
    argc: crate::ffi::c_int,
    argv: *const *const crate::ffi::c_char,
    _marker: PhantomData<&'a CStr>,
}

impl<'a> Args<'a> {
    /// Creates an argument wrapper from raw C `argc` and `argv`.
    ///
    /// # Safety
    ///
    /// `argv` must point to at least `argc` valid C string pointers.
    pub const unsafe fn from_raw(
        argc: crate::ffi::c_int,
        argv: *const *const crate::ffi::c_char,
    ) -> Self {
        Self {
            argc,
            argv,
            _marker: PhantomData,
        }
    }

    /// Returns the number of arguments.
    pub fn len(self) -> usize {
        if self.argc <= 0 {
            0
        } else {
            self.argc as usize
        }
    }

    /// Returns whether there are no arguments.
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Returns an argument by index.
    pub fn get(self, index: usize) -> Option<&'a CStr> {
        if index >= self.len() || self.argv.is_null() {
            return None;
        }

        let arg = unsafe { self.argv.add(index).read() };
        if arg.is_null() {
            None
        } else {
            Some(unsafe { CStr::from_ptr(arg) })
        }
    }

    /// Iterates over arguments.
    pub fn iter(self) -> ArgsIter<'a> {
        ArgsIter {
            args: self,
            index: 0,
        }
    }
}

/// Iterator over process arguments.
#[derive(Clone, Debug)]
pub struct ArgsIter<'a> {
    args: Args<'a>,
    index: usize,
}

impl<'a> Iterator for ArgsIter<'a> {
    type Item = &'a CStr;

    fn next(&mut self) -> Option<Self::Item> {
        let arg = self.args.get(self.index)?;
        self.index += 1;
        Some(arg)
    }
}

/// Converts an entry function return value into a process exit status.
pub trait Termination {
    /// Returns the process exit status.
    fn status(self) -> crate::ffi::c_int;
}

impl Termination for () {
    fn status(self) -> crate::ffi::c_int {
        0
    }
}

impl Termination for crate::ffi::c_int {
    fn status(self) -> crate::ffi::c_int {
        self
    }
}

impl Termination for crate::Result<()> {
    fn status(self) -> crate::ffi::c_int {
        match self {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }
}

/// Exits the process immediately.
pub fn exit(status: crate::ffi::c_int) -> ! {
    unsafe { crate::ffi::_exit(status) }
}

/// Defines a no_std process entry point for `main`.
///
/// This macro generates a global allocator, C ABI `main`, a panic handler, and
/// a `rust_eh_personality` shim for no_std binaries using prebuilt core/alloc.
///
/// ```rust,ignore
/// #![no_std]
/// #![no_main]
///
/// lunacy::entry!(main);
///
/// fn main() -> lunacy::Result<()> {
///     lunacy::fd::write_all(lunacy::fd::STDOUT, b"hello world\n")
/// }
/// ```
#[macro_export]
macro_rules! entry {
    ($main:path, args) => {
        #[global_allocator]
        static LUNACY_ALLOCATOR: $crate::LibcAllocator = $crate::LibcAllocator;

        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(export_name = "main")]
        pub extern "C" fn __lunacy_main(
            argc: $crate::ffi::c_int,
            argv: *const *const $crate::ffi::c_char,
        ) -> $crate::ffi::c_int {
            let args = unsafe { $crate::runtime::Args::from_raw(argc, argv) };
            $crate::runtime::Termination::status($main(args))
        }

        #[panic_handler]
        fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
            $crate::runtime::exit(101)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn rust_eh_personality() {}
    };

    ($main:path) => {
        #[global_allocator]
        static LUNACY_ALLOCATOR: $crate::LibcAllocator = $crate::LibcAllocator;

        #[allow(clippy::not_unsafe_ptr_arg_deref)]
        #[unsafe(export_name = "main")]
        pub extern "C" fn __lunacy_main(
            _argc: $crate::ffi::c_int,
            _argv: *const *const $crate::ffi::c_char,
        ) -> $crate::ffi::c_int {
            $crate::runtime::Termination::status($main())
        }

        #[panic_handler]
        fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
            $crate::runtime::exit(101)
        }

        #[unsafe(no_mangle)]
        pub extern "C" fn rust_eh_personality() {}
    };
}
