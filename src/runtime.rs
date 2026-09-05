/// Aborts through libc without unwinding or allocating.
#[doc(hidden)]
pub fn abort() -> ! {
    // SAFETY: abort has no preconditions and never returns.
    unsafe { crate::sys::abort() }
}

/// Supplies a C main entry point, libc global allocator, and aborting panic handler.
///
/// Invoke once at module scope in a `#![no_std]`, `#![no_main]` executable.
/// `lunacy_main!(entry)` calls `fn(Args<'_>) -> core::ffi::c_int`.
/// Use `fn entry(_: lunacy::Args<'_>) -> i32` to ignore arguments.
/// The return value becomes the process exit status. Build with `panic = "abort"`.
/// The system's normal C startup code initializes libc before this entry point.
///
/// This macro owns the executable's allocator and panic handler. Library users
/// and std executables should use the individual APIs without invoking it.
#[macro_export]
macro_rules! lunacy_main {
    ($entry:path) => {
        const _: () = {
            #[global_allocator]
            static ALLOCATOR: $crate::LibcAllocator = $crate::LibcAllocator;

            #[panic_handler]
            fn panic(_: &::core::panic::PanicInfo<'_>) -> ! {
                $crate::abort()
            }

            // Prebuilt core/alloc can retain this reference even when the
            // executable uses panic=abort. Unwinding is never supported here.
            #[unsafe(export_name = "rust_eh_personality")]
            extern "C" fn personality() -> ! {
                $crate::abort()
            }

            // Some prebuilt alloc functions retain unwind cleanup references
            // in debug builds. This runtime is abort-only, including this path.
            #[unsafe(export_name = "_Unwind_Resume")]
            extern "C" fn unwind_resume(_exception: *mut ::core::ffi::c_void) -> ! {
                $crate::abort()
            }

            #[unsafe(export_name = "main")]
            extern "C" fn __lunacy_main(
                argc: ::core::ffi::c_int,
                argv: *mut *mut ::core::ffi::c_char,
            ) -> ::core::ffi::c_int {
                let main: fn($crate::Args<'_>) -> ::core::ffi::c_int = $entry;
                // SAFETY: libc supplies argc valid strings. This view exists
                // only during main; foreign code must not mutate borrowed argv.
                let args = unsafe { $crate::Args::from_raw(argc as usize, argv.cast()) };
                main(args)
            }
        };
    };
}
