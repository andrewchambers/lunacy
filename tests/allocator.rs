use core::alloc::{GlobalAlloc, Layout};
use lunacy::allocator::LibcAllocator;

#[test]
fn allocation_alignment_and_zeroing() {
    for align in [1, 8, 16, 64, 4096] {
        for size in [1, 17, 129] {
            let layout = Layout::from_size_align(size, align).unwrap();
            // SAFETY: Nonzero layouts; each returned block is checked and freed
            // exactly once using its original layout.
            unsafe {
                let ptr = LibcAllocator.alloc(layout);
                assert!(!ptr.is_null());
                assert_eq!(ptr as usize % align, 0);
                ptr.write_bytes(0xa5, size);
                LibcAllocator.dealloc(ptr, layout);

                let ptr = LibcAllocator.alloc_zeroed(layout);
                assert!(!ptr.is_null());
                assert_eq!(ptr as usize % align, 0);
                assert!(
                    core::slice::from_raw_parts(ptr, size)
                        .iter()
                        .all(|&b| b == 0)
                );
                LibcAllocator.dealloc(ptr, layout);
            }
        }
    }
}

#[test]
fn reallocation_preserves_alignment_and_contents() {
    for align in [1, 16, 64, 4096] {
        let mut layout = Layout::from_size_align(37, align).unwrap();
        // SAFETY: Each reallocation receives the live block's current layout;
        // accesses are bounded by the current size and the last block is freed.
        unsafe {
            let mut ptr = LibcAllocator.alloc(layout);
            assert!(!ptr.is_null());
            ptr.write_bytes(0x5a, layout.size());
            for size in [8193, 13, 4097] {
                let next = LibcAllocator.realloc(ptr, layout, size);
                assert!(!next.is_null());
                assert_eq!(next as usize % align, 0);
                assert!(
                    core::slice::from_raw_parts(next, layout.size().min(size))
                        .iter()
                        .all(|&b| b == 0x5a)
                );
                ptr = next;
                layout = Layout::from_size_align(size, align).unwrap();
                ptr.write_bytes(0x5a, size);
            }
            LibcAllocator.dealloc(ptr, layout);
        }
    }
}
