use core::alloc::{GlobalAlloc, Layout};
use core::ptr;

/// A global allocator backed by the target's system libc.
///
/// This allocator uses `posix_memalign` so Rust allocation alignment
/// requirements are preserved, then releases memory with `free`.
///
/// ```rust,ignore
/// #![no_std]
///
/// extern crate alloc;
///
/// use lunacy::LibcAllocator;
///
/// #[global_allocator]
/// static ALLOCATOR: LibcAllocator = LibcAllocator;
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct LibcAllocator;

unsafe impl GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(1);
        let align = layout
            .align()
            .max(core::mem::size_of::<*mut crate::ffi::c_void>());
        let mut out = ptr::null_mut();

        let status = unsafe { crate::ffi::posix_memalign(&mut out, align, size) };
        if status == 0 {
            out.cast()
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        unsafe {
            crate::ffi::free(ptr.cast());
        }
    }
}
