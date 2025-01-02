use errors::ZeroCopyError;

pub mod cyclic_vec;
pub mod errors;
pub mod raw_pointer;
pub mod raw_pointer_mut;
pub mod slice_mut;
pub mod vec;

use std::mem::{align_of, size_of};

pub const SIZE_OF_ZERO_COPY_SLICE_METADATA: usize = 8;
pub const SIZE_OF_ZERO_COPY_VEC_METADATA: usize = 16;
pub const SIZE_OF_ZERO_COPY_CYCLIC_VEC_METADATA: usize = 24;
pub const DISCRIMINATOR_LEN: usize = 8;

pub fn is_aligned<T>(ptr: *const T) -> bool {
    (ptr as usize) % align_of::<T>() == 0
}

pub fn check_alignment<T>(ptr: *const T) -> Result<(), errors::ZeroCopyError> {
    if !is_aligned(ptr) {
        println!("Alignment mismatch: {}", (ptr as usize) % align_of::<T>());
        println!("align_of::<T>(): {}", align_of::<T>());
        return Err(errors::ZeroCopyError::UnalignedPointer);
    }
    Ok(())
}

pub fn size_is_ok<T>(data_size: usize) -> bool {
    data_size >= size_of::<T>()
}

pub fn check_size<T>(bytes: &[u8]) -> Result<(), ZeroCopyError> {
    if !size_is_ok::<T>(bytes.len()) {
        return Err(ZeroCopyError::InsufficientMemoryAllocated(
            bytes.len(),
            size_of::<T>(),
        ));
    }
    Ok(())
}

pub fn add_padding<LEN, T>(offset: &mut usize)
where
    LEN: Copy,
    T: Copy,
{
    let padding = align_of::<T>().saturating_sub(size_of::<LEN>());
    *offset += padding;
}
