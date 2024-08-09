use std::{alloc::Layout, cell::UnsafeCell, mem::size_of, ptr::null_mut};
pub mod bench;

#[cfg(target_os = "solana")]
use anchor_lang::{
    prelude::*,
    solana_program::entrypoint::{HEAP_LENGTH, HEAP_START_ADDRESS},
};

#[cfg(target_os = "solana")]
#[global_allocator]
pub static GLOBAL_ALLOCATOR: BumpAllocator = BumpAllocator {
    start: UnsafeCell::new(HEAP_START_ADDRESS as usize),
    len: HEAP_LENGTH,
};
// Implement Sync for BumpAllocator since Solana is single-threaded
unsafe impl Sync for BumpAllocator {
    // unimplemented!("Sync is not implemented for BumpAllocator");
}
#[cfg(target_os = "solana")]
#[error_code]
pub enum HeapError {
    #[msg("The provided position to free is invalid.")]
    InvalidHeapPos,
}
pub struct BumpAllocator {
    pub start: UnsafeCell<usize>,
    pub len: usize,
}

pub struct BumpAllocatorUsize {
    pub start: usize,
    pub len: usize,
}
impl BumpAllocatorUsize {
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
        println!("pos_ptr: {:?}", pos_ptr);
        println!("pos_ptr deref: {:?}", *pos_ptr);
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
    pub fn free_heap(&self, pos: usize) -> Result<()> {
        if pos < self.start + BumpAllocator::RESERVED_MEM || pos > self.start + self.len {
            return err!(HeapError::InvalidHeapPos);
        }

        unsafe { self.move_cursor(pos) };
        Ok(())
    }
}

unsafe impl std::alloc::GlobalAlloc for BumpAllocatorUsize {
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
impl BumpAllocator {
    const RESERVED_MEM: usize = size_of::<*mut u8>();

    #[cfg(target_os = "solana")]
    pub fn new() -> Self {
        Self {
            start: UnsafeCell::new(HEAP_START_ADDRESS as usize),
            len: HEAP_LENGTH,
        }
    }

    /// Returns the current position of the heap.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it returns a raw pointer.
    pub unsafe fn pos(&self) -> usize {
        let pos_ptr = (*self.start.get()) as *mut usize;
        println!("pos_ptr: {:?}", pos_ptr);
        let pos_value = *pos_ptr;
        println!("pos_ptr deref: {:?}", pos_value);
        pos_value as usize
    }

    /// Reset heap start cursor to position.
    ///
    /// # Safety
    ///
    /// Do not use this function if you initialized heap memory after pos which you still need.
    pub unsafe fn move_cursor(&self, pos: usize) {
        let pos_ptr = self.start.get();
        println!("pos_ptr: {:?}", pos_ptr);
        println!("pos: {:?}", pos);
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
    pub fn free_heap(&self, pos: usize) -> Result<()> {
        unsafe {
            if pos < *((*self.start.get()) as *mut usize) + BumpAllocator::RESERVED_MEM
                || pos > *((*self.start.get()) as *mut usize) + self.len
            {
                return err!(HeapError::InvalidHeapPos);
            }

            self.move_cursor(pos)
        }
        Ok(())
    }
}

unsafe impl std::alloc::GlobalAlloc for BumpAllocator {
    // #[inline]
    // unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    //     let pos_ptr = self.start.get();

    //     let mut pos = *pos_ptr;
    //     if pos == 0 {
    //         // First time, set starting position
    //         pos = *((*self.start.get()) as *mut usize) + self.len;
    //     }
    //     pos = pos.saturating_sub(layout.size());
    //     pos &= !(layout.align().wrapping_sub(1));
    //     if pos < *((*self.start.get()) as *mut usize) + BumpAllocator::RESERVED_MEM {
    //         return null_mut();
    //     }
    //     *pos_ptr = pos;
    //     pos as *mut u8
    // }
    #[inline]
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let pos_ptr = self.start.get(); // Correctly get the raw pointer

        let mut pos = *pos_ptr; // Dereference to get the current position
        println!("pos: {}", pos);
        if 0 == *((*self.start.get()) as *mut usize) {
            // Check if it's the first allocation
            pos = *self.start.get() + self.len;
            println!("pos if: {}", pos);
        }
        println!("len {}", self.len);
        println!("(layout.size() {}", layout.size());

        pos = pos.saturating_sub(layout.size());
        println!("pos sat: {}", pos);

        pos &= !(layout.align().wrapping_sub(1));
        println!("pos&: {}", pos);

        if pos < self.start.get() as usize + BumpAllocator::RESERVED_MEM {
            return null_mut();
        }
        println!("pos: {}", pos);
        *pos_ptr = pos; // Correctly update the position
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
    fn test_unsafe_cell() {
        let heap = [0u8; 128];
        let ptr = heap.as_ptr() as *const _ as usize;
        let cell = UnsafeCell::new(ptr);
        // unsafe {
        //     assert_eq!(ptr, *cell.get());
        // }
        let alloc_usize = BumpAllocatorUsize {
            start: ptr,
            len: heap.len(),
        };
        let alloc = BumpAllocator {
            start: cell,
            len: heap.len(),
        };
        println!("heap: {:?}", heap.as_ptr() as *const _ as usize);
        unsafe {
            assert_eq!(ptr, *alloc.start.get() as usize);
            assert_eq!(ptr, alloc_usize.start);
        }
        unsafe {
            assert_eq!(alloc_usize.start, *alloc.start.get() as usize);
            assert_eq!(alloc_usize.pos(), alloc.pos());
        }
        let pos = unsafe { alloc_usize.pos() };
        assert_eq!(pos, unsafe { alloc_usize.pos() });
        println!("pos: {}", pos);
        assert_eq!(pos, 0);
        let pos = unsafe { alloc.pos() };
        assert_eq!(pos, unsafe { alloc.pos() });
        println!("pos: {}", pos);
        assert_eq!(pos, 0);
    }

    #[test]
    fn test_pos_move_cursor_heap() {
        use std::mem::size_of;

        {
            let heap = [0u8; 128];
            let allocator = BumpAllocator {
                start: (heap.as_ptr() as *const _ as usize).into(),
                len: heap.len(),
            };
            let pos = unsafe { allocator.pos() };
            assert_eq!(pos, unsafe { allocator.pos() });
            println!("pos: {}", pos);
            println!("heap: {:?}", heap.as_ptr() as *const _ as usize);
            unsafe {
                println!(
                    "allocator start: {:?}",
                    allocator.start.get() as *const _ as usize
                );
                println!("allocator start deref: {:?}", *allocator.start.get());
            }
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
            println!("pos_128: {}", pos_128);
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
                start: UnsafeCell::new(heap.as_ptr() as *const _ as usize),
                len: heap.len(),
            };
            println!("heap: {:?}", heap.as_ptr() as *const _ as usize);
            for i in 0..128 - size_of::<*mut u8>() {
                println!("i: {}", i);
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
                start: (heap.as_ptr() as *const _ as usize).into(),
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
                start: (heap.as_ptr() as *const _ as usize).into(),
                len: heap.len(),
            };
            let ptr =
                unsafe { allocator.alloc(Layout::from_size_align(120, size_of::<u8>()).unwrap()) };
            assert_ne!(ptr, null_mut());
            assert_eq!(0, ptr.align_offset(size_of::<u64>()));
        }
    }
}
