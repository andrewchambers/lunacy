//! Rust allocation backed by the system libc heap.

use core::alloc::{GlobalAlloc, Layout};

use crate::sys;

/// A global allocator backed by malloc, calloc, realloc, and free.
///
/// Larger alignments use posix_memalign; reallocating those blocks allocates,
/// copies, and frees. Failures return null and preserve the old allocation.
/// Merely depending on lunacy does not register this allocator.
///
/// ```
/// #[global_allocator]
/// static ALLOCATOR: lunacy::LibcAllocator = lunacy::LibcAllocator;
/// ```
pub struct LibcAllocator;

// SAFETY: The C shim satisfies Layout's size/alignment, preserves contents on
// realloc, and leaves allocations intact on failure. libc supplies thread safety.
// These methods do not unwind or call Rust allocation routines.
unsafe impl GlobalAlloc for LibcAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc callers provide a valid, nonzero layout.
        unsafe { sys::lunacy_alloc(layout.size(), layout.align()).cast() }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        // SAFETY: All allocations from this allocator can be released with free.
        unsafe { sys::free(ptr.cast()) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Same layout requirements as alloc; the shim zeroes every byte.
        unsafe { sys::lunacy_alloc_zeroed(layout.size(), layout.align()).cast() }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: Caller provides a live allocation, its layout, and valid size.
        unsafe { sys::lunacy_realloc(ptr.cast(), layout.size(), layout.align(), new_size).cast() }
    }
}
