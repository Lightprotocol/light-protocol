use std::{alloc::Layout, mem::size_of, ptr::null_mut, usize};

#[cfg(target_os = "solana")]
use anchor_lang::{
    prelude::*,
    solana_program::entrypoint::{HEAP_LENGTH, HEAP_START_ADDRESS},
};

#[cfg(target_os = "solana")]
#[global_allocator]
pub static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator {
    start: HEAP_START_ADDRESS as usize,
    len: HEAP_LENGTH,
};

pub struct BumpAllocator {
    pub start: usize,
    pub len: usize,
}

impl BumpAllocator {
    const RESERVED_MEM: usize = size_of::<*mut u8>();

    #[cfg(target_os = "solana")]
    pub fn new() -> Self {
        Self {
            start: HEAP_START_ADDRESS as usize,
            len: HEAP_LENGTH,
        }
    }

    /// Returns the current position of the heap.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn pos(&self) -> usize {
        let pos_ptr = self.start as *mut usize;
        *pos_ptr
    }

    /// Reset heap start cursor to position.
    ///
    /// # Safety
    ///
    /// Do not use this function if you initialized heap memory after pos which you still need.
    pub unsafe fn move_cursor(&self, pos: usize) {
        let pos_ptr = self.start as *mut usize;
        *pos_ptr = pos;
    }

    #[cfg(target_os = "solana")]
    pub fn log_total_heap(&self, msg: &str) -> u64 {
        const HEAP_END_ADDRESS: u64 = HEAP_START_ADDRESS as u64 + HEAP_LENGTH as u64;

        let heap_start = unsafe { self.pos() } as u64;
        let heap_used = HEAP_END_ADDRESS - heap_start;
        msg!("{}: total heap used: {}", msg, heap_used);
        heap_used
    }

    #[cfg(target_os = "solana")]
    pub fn get_heap_pos(&self) -> usize {
        let heap_start = unsafe { self.pos() } as usize;
        heap_start
    }

    #[cfg(target_os = "solana")]
    pub fn free_heap(&self, pos: usize) {
        unsafe { self.move_cursor(pos) };
    }
}

unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pos_ptr = self.start as *mut usize;

        let mut pos = *pos_ptr;
        if pos == 0 {
            // First time, set starting position
            pos = self.start + self.len;
        }
        pos = pos.saturating_sub(layout.size());
        pos &= !(layout.align().wrapping_sub(1));
        if pos < self.start + BumpAllocator::RESERVED_MEM {
            return null_mut();
        }
        *pos_ptr = pos;
        pos as *mut u8
    }
    #[inline]
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {
        // no dellaoc in Solana runtime :*(
    }
}

#[cfg(test)]
mod test {
    use std::{
        alloc::{GlobalAlloc, Layout},
        mem::size_of,
        ptr::null_mut,
    };

    use super::*;

    #[test]
    fn test_pos_move_cursor_heap() {
        use std::mem::size_of;

        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let pos = unsafe { allocator.pos() };
            assert_eq!(pos, unsafe { allocator.pos() });
            assert_eq!(pos, 0);
            let mut pos_64 = 0;
            for i in 0..128 - size_of::<*mut u8>() {
                if i == 64 {
                    pos_64 = unsafe { allocator.pos() };
                }
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - i
                );
                assert_eq!(ptr as *const _ as usize, unsafe { allocator.pos() });
            }
            let pos_128 = unsafe { allocator.pos() };
            // free half of the heap
            unsafe { allocator.move_cursor(pos_64) };
            assert_eq!(pos_64, unsafe { allocator.pos() });
            assert_ne!(pos_64 + 1, unsafe { allocator.pos() });
            // allocate second half of the heap again
            for i in 0..64 - size_of::<*mut u8>() {
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - (i + 64)
                );
                assert_eq!(ptr as *const _ as usize, unsafe { allocator.pos() });
            }
            assert_eq!(pos_128, unsafe { allocator.pos() });
            // free all of the heap
            unsafe { allocator.move_cursor(pos) };
            assert_eq!(pos, unsafe { allocator.pos() });
            assert_ne!(pos + 1, unsafe { allocator.pos() });
        }
    }

    /// taken from solana-program https://github.com/solana-labs/solana/blob/9a520fd5b42bafefa4815afe3e5390b4ea7482ca/sdk/program/src/entrypoint.rs#L374
    #[test]
    fn test_bump_allocator() {
        // alloc the entire
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            for i in 0..128 - size_of::<*mut u8>() {
                let ptr = unsafe {
                    allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap())
                };
                assert_eq!(
                    ptr as *const _ as usize,
                    heap.as_ptr() as *const _ as usize + heap.len() - 1 - i
                );
            }
            assert_eq!(null_mut(), unsafe {
                allocator.alloc(Layout::from_size_align(1, 1).unwrap())
            });
        }
        // check alignment
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u8>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u8>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u16>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u16>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u32>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u32>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u64>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u64>()));
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(1, size_of::<u128>()).unwrap()) };
            assert_eq!(0, ptr.align_offset(size_of::<u128>()));
            let ptr = unsafe { allocator.alloc(Layout::from_size_align(1, 64).unwrap()) };
            assert_eq!(0, ptr.align_offset(64));
        }
        // alloc entire block (minus the pos ptr)
        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: heap.as_ptr() as *const _ as usize,
                len: heap.len(),
            };
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(120, size_of::<u8>()).unwrap()) };
            assert_ne!(ptr, null_mut());
            assert_eq!(0, ptr.align_offset(size_of::<u64>()));
        }
    }
}
