use std::{mem, ptr};

use light_bounded_vec::{
    BoundedVec, BoundedVecMetadata, CyclicBoundedVec, CyclicBoundedVecMetadata,
};

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

    let mut vec = BoundedVec::with_metadata(metadata);
    let dst_ptr: *mut T = vec.as_mut_ptr();

    for i in 0..metadata.length() {
        let val = ptr::read(ptr.add(i));
        // SAFETY: We ensured the bounds.
        unsafe { ptr::write(dst_ptr.add(i), val) };
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
    let src_ptr = bytes[*offset..*offset + size].as_ptr() as *const T;

    let mut vec = CyclicBoundedVec::with_metadata(metadata);
    let dst_ptr: *mut T = vec.as_mut_ptr();

    for i in 0..metadata.length() {
        let val = ptr::read(src_ptr.add(i));
        // SAFETY: We ensured the bounds.
        unsafe { ptr::write(dst_ptr.add(i), val) };
    }

    *offset += size;

    vec
}

#[cfg(test)]
mod test {
    use super::*;

    use bytemuck::{Pod, Zeroable};
    use memoffset::offset_of;

    #[test]
    fn test_value_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: isize,
            b: usize,
            c: i64,
            d: u64,
            e: i32,
            f: u32,
            g: i16,
            _padding_1: [u8; 2],
            h: u16,
            _padding_2: [u8; 2],
            i: i8,
            _padding_3: [i8; 3],
            j: u8,
            _padding_4: [i8; 3],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];
        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            (*s).a = -9223372036854771632;
            (*s).b = 9223372036854771632;
            (*s).c = -9223372036854771632;
            (*s).d = 9223372036854771632;
            (*s).e = -2147483623;
            (*s).f = 2147483623;
            (*s).g = -32721;
            (*s).h = 32721;
            (*s).i = -127;
            (*s).j = 127;

            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            assert_eq!(
                read_value_at::<isize>(&buf, &mut offset),
                -9223372036854771632
            );
            assert_eq!(offset, 8);

            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 8);
            assert_eq!(
                read_value_at::<usize>(&buf, &mut offset),
                9223372036854771632
            );
            assert_eq!(offset, 16);

            let mut offset = offset_of!(TestStruct, c);
            assert_eq!(offset, 16);
            assert_eq!(
                read_value_at::<i64>(&buf, &mut offset),
                -9223372036854771632
            );
            assert_eq!(offset, 24);

            let mut offset = offset_of!(TestStruct, d);
            assert_eq!(offset, 24);
            assert_eq!(read_value_at::<u64>(&buf, &mut offset), 9223372036854771632);
            assert_eq!(offset, 32);

            let mut offset = offset_of!(TestStruct, e);
            assert_eq!(offset, 32);
            assert_eq!(read_value_at::<i32>(&buf, &mut offset), -2147483623);
            assert_eq!(offset, 36);

            let mut offset = offset_of!(TestStruct, f);
            assert_eq!(offset, 36);
            assert_eq!(read_value_at::<u32>(&buf, &mut offset), 2147483623);
            assert_eq!(offset, 40);

            let mut offset = offset_of!(TestStruct, g);
            assert_eq!(offset, 40);
            assert_eq!(read_value_at::<i16>(&buf, &mut offset), -32721);
            assert_eq!(offset, 42);

            let mut offset = offset_of!(TestStruct, h);
            assert_eq!(offset, 44);
            assert_eq!(read_value_at::<u16>(&buf, &mut offset), 32721);
            assert_eq!(offset, 46);

            let mut offset = offset_of!(TestStruct, i);
            assert_eq!(offset, 48);
            assert_eq!(read_value_at::<i8>(&buf, &mut offset), -127);
            assert_eq!(offset, 49);

            let mut offset = offset_of!(TestStruct, j);
            assert_eq!(offset, 52);
            assert_eq!(read_value_at::<u8>(&buf, &mut offset), 127);
            assert_eq!(offset, 53);
        }
    }

    #[test]
    fn test_read_bounded_vec_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: [i64; 32],
            b: [u64; 32],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];
        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            for (i, element) in (*s).a.iter_mut().enumerate() {
                *element = -(i as i64);
            }
            for (i, element) in (*s).b.iter_mut().enumerate() {
                *element = i as u64;
            }

            let metadata = BoundedVecMetadata::new_with_length(32, 32);
            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            let vec: BoundedVec<i64> = read_bounded_vec_at(&buf, &mut offset, &metadata);
            for (i, element) in vec.iter().enumerate() {
                assert_eq!(i as i64, -(*element as i64));
            }
        }
    }

    #[test]
    fn test_read_cyclic_bounded_vec_at() {
        #[derive(Clone, Copy, Pod, Zeroable)]
        #[repr(C)]
        struct TestStruct {
            a: [i64; 32],
            b: [u64; 32],
        }

        let mut buf = vec![0_u8; mem::size_of::<TestStruct>()];
        let s = buf.as_mut_ptr() as *mut TestStruct;

        unsafe {
            for (i, element) in (*s).a.iter_mut().enumerate() {
                *element = -(i as i64);
            }
            for (i, element) in (*s).b.iter_mut().enumerate() {
                *element = i as u64;
            }

            // Start the cyclic vec from the middle.
            let metadata = CyclicBoundedVecMetadata::new_with_indices(32, 32, 14, 13);
            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            let vec: CyclicBoundedVec<i64> =
                read_cyclic_bounded_vec_at(&buf, &mut offset, &metadata);
            assert_eq!(vec.capacity(), 32);
            assert_eq!(vec.len(), 32);
            assert_eq!(vec.first_index(), 14);
            assert_eq!(vec.last_index(), 13);
            assert_eq!(
                vec.iter().collect::<Vec<_>>().as_slice(),
                &[
                    &-14, &-15, &-16, &-17, &-18, &-19, &-20, &-21, &-22, &-23, &-24, &-25, &-26,
                    &-27, &-28, &-29, &-30, &-31, &-0, &-1, &-2, &-3, &-4, &-5, &-6, &-7, &-8, &-9,
                    &-10, &-11, &-12, &-13
                ]
            );
        }
    }
}
