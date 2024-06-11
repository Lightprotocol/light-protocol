use std::{mem, ptr};

use light_bounded_vec::{
    BoundedVec, BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecMetadata,
};

/// Casts a part of provided `bytes` buffer with the given `offset` to a
/// mutable pointer to `T`.
///
/// Should be used for single values.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_ptr_at<T>(bytes: &[u8], offset: &mut usize) -> *mut T {
    let size = mem::size_of::<T>();
    let ptr = bytes[*offset..*offset + size].as_ptr() as *mut T;
    *offset += size;
    ptr
}

/// Casts a part of provided `bytes` buffer with the given `offset` to a
/// mutable pointer to `T`.
///
/// Should be used for array-type sequences.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_array_like_ptr_at<T>(bytes: &[u8], offset: &mut usize, len: usize) -> *mut T {
    let size = mem::size_of::<T>() * len;
    let ptr = bytes[*offset..*offset + size].as_ptr() as *mut T;
    *offset += size;
    ptr
}

/// Creates a copy of value of type `T` based on the provided `bytes` buffer.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_value_at<T>(bytes: &[u8], offset: &mut usize) -> T
where
    T: Clone,
{
    let size = mem::size_of::<T>();
    let ptr = bytes[*offset..*offset + size].as_ptr() as *const T;
    *offset += size;
    // (*ptr).clone()
    ptr::read(ptr)
}

/// Creates a `BoundedVec` from the sequence of values provided in `bytes` buffer.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_bounded_vec_at<T>(
    bytes: &[u8],
    offset: &mut usize,
    metadata: &BoundedVecMetadata,
) -> BoundedVec<T>
where
    T: Clone,
{
    let size = mem::size_of::<T>() * metadata.capacity();
    let ptr = bytes[*offset..*offset + size].as_ptr() as *const T;

    let mut vec = BoundedVec::with_capacity(metadata.capacity());
    for i in 0..metadata.length() {
        let val = ptr::read(ptr.add(i));
        // PANICS: We ensured the bounds.
        vec.push(val).unwrap();
    }

    *offset += size;

    vec
}

/// Creates a `CyclicBoundedVec` from the sequence of values provided in
/// `bytes` buffer.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
pub unsafe fn read_cyclic_bounded_vec_at<T>(
    bytes: &[u8],
    offset: &mut usize,
    metadata: &CyclicBoundedVecMetadata,
) -> CyclicBoundedVec<T>
where
    T: Clone,
{
    let size = mem::size_of::<T>() * metadata.capacity();
    let ptr = bytes[*offset..*offset + size].as_ptr() as *const T;

    let mut vec = CyclicBoundedVec::with_capacity(metadata.capacity());
    for i in 0..metadata.length() {
        let val = ptr::read(ptr.add(i));
        vec.push(val);
    }

    *offset += size;

    vec
}

/// Writes provided `data` into provided `bytes` buffer with the given
/// `offset`.
pub fn write_at<T>(bytes: &mut [u8], data: &[u8], offset: &mut usize) {
    let size = mem::size_of::<T>();
    bytes[*offset..*offset + size].copy_from_slice(data);
    *offset += size;
}
