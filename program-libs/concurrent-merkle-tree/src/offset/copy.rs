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
    ptr::read(ptr)
}

/// Creates a `BoundedVec` from the sequence of values provided in `bytes` buffer.
///
/// # Safety
///
/// This is higly unsafe. This function doesn't ensure alignment and
/// correctness of provided buffer. The responsibility of such checks is on
/// the caller.
///
/// The `T` type needs to be either a primitive or struct consisting of
/// primitives. It cannot contain any nested heap-backed stucture (like vectors,
/// slices etc.).
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
    use std::slice;

    use memoffset::offset_of;

    use super::*;

    #[test]
    fn test_value_at() {
        #[derive(Clone, Copy)]
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
            (*s).a = isize::MIN;
            (*s).b = usize::MAX;
            (*s).c = i64::MIN;
            (*s).d = u64::MAX;
            (*s).e = i32::MIN;
            (*s).f = u32::MAX;
            (*s).g = i16::MIN;
            (*s).h = u16::MAX;
            (*s).i = i8::MIN;
            (*s).j = u8::MAX;

            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            assert_eq!(read_value_at::<isize>(&buf, &mut offset), isize::MIN);
            assert_eq!(offset, 8);

            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 8);
            assert_eq!(read_value_at::<usize>(&buf, &mut offset), usize::MAX);
            assert_eq!(offset, 16);

            let mut offset = offset_of!(TestStruct, c);
            assert_eq!(offset, 16);
            assert_eq!(read_value_at::<i64>(&buf, &mut offset), i64::MIN);
            assert_eq!(offset, 24);

            let mut offset = offset_of!(TestStruct, d);
            assert_eq!(offset, 24);
            assert_eq!(read_value_at::<u64>(&buf, &mut offset), u64::MAX);
            assert_eq!(offset, 32);

            let mut offset = offset_of!(TestStruct, e);
            assert_eq!(offset, 32);
            assert_eq!(read_value_at::<i32>(&buf, &mut offset), i32::MIN);
            assert_eq!(offset, 36);

            let mut offset = offset_of!(TestStruct, f);
            assert_eq!(offset, 36);
            assert_eq!(read_value_at::<u32>(&buf, &mut offset), u32::MAX);
            assert_eq!(offset, 40);

            let mut offset = offset_of!(TestStruct, g);
            assert_eq!(offset, 40);
            assert_eq!(read_value_at::<i16>(&buf, &mut offset), i16::MIN);
            assert_eq!(offset, 42);

            let mut offset = offset_of!(TestStruct, h);
            assert_eq!(offset, 44);
            assert_eq!(read_value_at::<u16>(&buf, &mut offset), u16::MAX);
            assert_eq!(offset, 46);

            let mut offset = offset_of!(TestStruct, i);
            assert_eq!(offset, 48);
            assert_eq!(read_value_at::<i8>(&buf, &mut offset), i8::MIN);
            assert_eq!(offset, 49);

            let mut offset = offset_of!(TestStruct, j);
            assert_eq!(offset, 52);
            assert_eq!(read_value_at::<u8>(&buf, &mut offset), u8::MAX);
            assert_eq!(offset, 53);
        }
    }

    #[test]
    fn test_read_bounded_vec_at() {
        #[derive(Clone, Copy)]
        #[repr(C)]
        struct TestStruct {
            a: [i64; 32],
            b: [u64; 32],
            c: [[u8; 32]; 32],
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
            for (i, element) in (*s).c.iter_mut().enumerate() {
                *element = [i as u8; 32];
            }

            let metadata = BoundedVecMetadata::new_with_length(32, 32);
            let mut offset = offset_of!(TestStruct, a);
            assert_eq!(offset, 0);
            let vec: BoundedVec<i64> = read_bounded_vec_at(&buf, &mut offset, &metadata);
            for (i, element) in vec.iter().enumerate() {
                assert_eq!(i as i64, -*element);
            }
            assert_eq!(offset, 256);

            let metadata = BoundedVecMetadata::new_with_length(32, 32);
            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 256);
            let vec: BoundedVec<u64> = read_bounded_vec_at(&buf, &mut offset, &metadata);
            for (i, element) in vec.iter().enumerate() {
                assert_eq!(i as u64, *element);
            }
            assert_eq!(offset, 512);

            let metadata = BoundedVecMetadata::new_with_length(32, 32);
            let mut offset = offset_of!(TestStruct, c);
            assert_eq!(offset, 512);
            let vec: BoundedVec<[u8; 32]> = read_bounded_vec_at(&buf, &mut offset, &metadata);
            for (i, element) in vec.iter().enumerate() {
                assert_eq!(&[i as u8; 32], element);
            }
            assert_eq!(offset, 1536);
        }
    }

    #[test]
    fn test_read_cyclic_bounded_vec_at() {
        #[derive(Clone, Copy)]
        #[repr(C)]
        struct TestStruct {
            a: [i64; 32],
            b: [u64; 32],
            c: [[u8; 32]; 32],
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
            for (i, element) in (*s).c.iter_mut().enumerate() {
                *element = [i as u8; 32];
            }

            // Start the cyclic vecs from the middle.

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
            assert_eq!(offset, 256);

            let metadata = CyclicBoundedVecMetadata::new_with_indices(32, 32, 14, 13);
            let mut offset = offset_of!(TestStruct, b);
            assert_eq!(offset, 256);
            let vec: CyclicBoundedVec<u64> =
                read_cyclic_bounded_vec_at(&buf, &mut offset, &metadata);
            assert_eq!(vec.capacity(), 32);
            assert_eq!(vec.len(), 32);
            assert_eq!(vec.first_index(), 14);
            assert_eq!(vec.last_index(), 13);
            assert_eq!(
                vec.iter().collect::<Vec<_>>().as_slice(),
                &[
                    &14, &15, &16, &17, &18, &19, &20, &21, &22, &23, &24, &25, &26, &27, &28, &29,
                    &30, &31, &0, &1, &2, &3, &4, &5, &6, &7, &8, &9, &10, &11, &12, &13
                ]
            );
            assert_eq!(offset, 512);

            let metadata = CyclicBoundedVecMetadata::new_with_indices(32, 32, 14, 13);
            let mut offset = offset_of!(TestStruct, c);
            assert_eq!(offset, 512);
            let vec: CyclicBoundedVec<[u8; 32]> =
                read_cyclic_bounded_vec_at(&buf, &mut offset, &metadata);
            assert_eq!(vec.capacity(), 32);
            assert_eq!(vec.len(), 32);
            assert_eq!(vec.first_index(), 14);
            assert_eq!(vec.last_index(), 13);
            assert_eq!(
                vec.iter().collect::<Vec<_>>().as_slice(),
                &[
                    &[14_u8; 32],
                    &[15_u8; 32],
                    &[16_u8; 32],
                    &[17_u8; 32],
                    &[18_u8; 32],
                    &[19_u8; 32],
                    &[20_u8; 32],
                    &[21_u8; 32],
                    &[22_u8; 32],
                    &[23_u8; 32],
                    &[24_u8; 32],
                    &[25_u8; 32],
                    &[26_u8; 32],
                    &[27_u8; 32],
                    &[28_u8; 32],
                    &[29_u8; 32],
                    &[30_u8; 32],
                    &[31_u8; 32],
                    &[0_u8; 32],
                    &[1_u8; 32],
                    &[2_u8; 32],
                    &[3_u8; 32],
                    &[4_u8; 32],
                    &[5_u8; 32],
                    &[6_u8; 32],
                    &[7_u8; 32],
                    &[8_u8; 32],
                    &[9_u8; 32],
                    &[10_u8; 32],
                    &[11_u8; 32],
                    &[12_u8; 32],
                    &[13_u8; 32],
                ]
            );
            assert_eq!(offset, 1536);
        }
    }

    #[test]
    fn test_read_cyclic_bounded_vec_first_last() {
        let mut vec = CyclicBoundedVec::<u32>::with_capacity(2);
        vec.push(0);
        vec.push(37);
        vec.push(49);

        let metadata_bytes = vec.metadata().to_le_bytes();
        let metadata = CyclicBoundedVecMetadata::from_le_bytes(metadata_bytes);
        let bytes = unsafe {
            slice::from_raw_parts(
                vec.as_mut_ptr() as *mut u8,
                mem::size_of::<u32>() * vec.capacity(),
            )
        };

        let mut offset = 0;
        let vec_copy: CyclicBoundedVec<u32> =
            unsafe { read_cyclic_bounded_vec_at(bytes, &mut offset, &metadata) };

        assert_eq!(*vec.first().unwrap(), 37);
        assert_eq!(vec.first(), vec_copy.first()); // Fails. Both should be 37
        assert_eq!(*vec.last().unwrap(), 49);
        assert_eq!(vec.last(), vec_copy.last()); // Fails. Both should be 49
    }
}
