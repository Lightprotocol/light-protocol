#![cfg(feature = "std")]
use core::fmt::Debug;

use light_zero_copy::{
    add_padding,
    errors::ZeroCopyError,
    slice::{ZeroCopySlice, ZeroCopySliceBorsh, ZeroCopySliceU64},
    slice_mut::{ZeroCopySliceMut, ZeroCopySliceMutU64},
};
use rand::{distributions::Standard, prelude::*};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
};

fn test_zero_copy_slice_new<LEN, T>(length: LEN, length_bytes: Vec<u8>)
where
    LEN: Copy + FromBytes + Immutable + KnownLayout + IntoBytes,
    T: Copy + Clone + PartialEq + Debug + Default + FromBytes + Immutable + KnownLayout + IntoBytes,
    u64: From<LEN>,
    Standard: Distribution<T>,
{
    let mut rng = thread_rng();
    let mut data = vec![0; ZeroCopySlice::<LEN, T>::required_size_for_capacity(length)];
    data[0..size_of::<LEN>()].copy_from_slice(&length_bytes);
    ZeroCopySlice::<LEN, T>::from_bytes(&data).unwrap();
    let usize_len: usize = u64::from(length) as usize;

    // Test from_bytes with a zeroed slice
    {
        let reference_vec = vec![T::default(); usize_len];
        println!("data len {}", data.len());
        let slice = ZeroCopySlice::<LEN, T>::from_bytes(&data).unwrap();
        // 1. Validate length
        assert_eq!(slice.len(), reference_vec.len());

        // 2. Validate slice content
        assert_eq!(slice.as_slice(), reference_vec.as_slice());

        // 3. Validate is not_empty
        assert!(!slice.is_empty());

        // 4. Validate accessors return None
        assert_eq!(T::default(), *slice.first().unwrap());
        assert_eq!(T::default(), *slice.last().unwrap());
        {
            let data = data.clone();
            let mut metadata_size = size_of::<LEN>();
            let mut length_bytes = data[0..metadata_size].to_vec();
            // pad with zeros until we have 8 bytes
            while length_bytes.len() < 8 {
                length_bytes.push(0);
            }
            assert_eq!(length_bytes, (u64::from(length).to_le_bytes()).to_vec());

            let padding_start = metadata_size;
            add_padding::<LEN, T>(&mut metadata_size);
            let padding_end = metadata_size;
            let data = data[padding_start..padding_end].to_vec();
            // Padding should be zeroed
            assert_eq!(data, vec![0; padding_end - padding_start]);
        }
    }
    let mut reference_vec: Vec<T> = vec![];
    // fill vector
    {
        let slice_length_bytes = &mut data[0..size_of::<LEN>()];
        slice_length_bytes.copy_from_slice(&length_bytes);
        for chunk in data[ZeroCopySlice::<LEN, T>::metadata_size()..].chunks_mut(size_of::<T>()) {
            let value = rng.gen();
            chunk.copy_from_slice(value.as_bytes());
            reference_vec.push(value);
        }
    }
    // Test from_bytes with a filled slice
    {
        let slice = ZeroCopySlice::<LEN, T>::from_bytes(&data).unwrap();
        // 1. Validate length
        assert_eq!(slice.len(), reference_vec.len());

        // 2. Validate slice content
        assert_eq!(slice.as_slice(), reference_vec.as_slice());

        // 3. Validate is not_empty
        assert!(!slice.is_empty());

        // 4. Validate accessors return None
        assert_eq!(reference_vec.first().unwrap(), slice.first().unwrap());
        assert_eq!(reference_vec.last().unwrap(), slice.last().unwrap());

        for (element, value) in slice.iter().zip(&reference_vec) {
            assert_eq!(element, value);
        }

        for (i, element) in reference_vec.iter().enumerate() {
            assert_eq!(*element, *slice.get(i).unwrap());
        }
    }
}

#[test]
fn test_zero_copy_slice() {
    test_zero_copy_slice_new::<u8, u32>(u8::MAX, u8::MAX.to_le_bytes().to_vec());
    println!("test_zero_copy_vec_with_capacity::<u8>()");
    test_zero_copy_slice_new::<U16, u32>(
        (u8::MAX as u16).into(),
        (u8::MAX as u16).to_le_bytes().to_vec(),
    );
    println!("test_zero_copy_vec_with_capacity::<u16>()");
    test_zero_copy_slice_new::<U32, u32>(
        (u8::MAX as u32).into(),
        (u8::MAX as u32).to_le_bytes().to_vec(),
    );
    println!("test_zero_copy_vec_with_capacity::<u32>()");
    test_zero_copy_slice_new::<U64, u32>(
        (u8::MAX as u64).into(),
        (u8::MAX as u64).to_le_bytes().to_vec(),
    );
    println!("test_zero_copy_vec_with_capacity::<u64>()");
}

#[test]
fn test_zero_copy_unaligned_type_for_len() {
    #[derive(
        Copy, Clone, PartialEq, Debug, Default, Immutable, FromBytes, KnownLayout, IntoBytes,
    )]
    #[repr(C)]
    struct TestStruct {
        a: u32,
        aa: u32,
        b: u64,
        c: u16,
        cc: u16,
        ccc: u32,
        d: [u8; 32],
    }
    impl Distribution<TestStruct> for Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestStruct {
            TestStruct {
                a: rng.gen(),
                aa: rng.gen(),
                b: rng.gen(),
                c: rng.gen(),
                cc: rng.gen(),
                ccc: rng.gen(),
                d: rng.gen(),
            }
        }
    }

    test_zero_copy_slice_new::<u8, TestStruct>(u8::MAX, u8::MAX.to_le_bytes().to_vec());
    test_zero_copy_slice_new::<U16, TestStruct>(
        (u8::MAX as u16).into(),
        (u8::MAX as u16).to_le_bytes().to_vec(),
    );
    test_zero_copy_slice_new::<U32, TestStruct>(
        (u8::MAX as u32).into(),
        (u8::MAX as u32).to_le_bytes().to_vec(),
    );
    test_zero_copy_slice_new::<U64, TestStruct>(
        (u8::MAX as u64).into(),
        (u8::MAX as u64).to_le_bytes().to_vec(),
    );
}

#[test]
fn test_unaligned() {
    #[derive(
        Copy, Clone, PartialEq, Debug, Default, Immutable, FromBytes, KnownLayout, IntoBytes,
    )]
    #[repr(C)]
    struct TestStruct {
        a: u32,
        aa: u32,
        b: u64,
        c: u16,
        cc: u16,
        ccc: u32,
        d: [u8; 32],
    }
    {
        let mut data =
            vec![0; ZeroCopySlice::<u8, TestStruct, false>::required_size_for_capacity(1)];
        data[0] = 1;
        let result = ZeroCopySlice::<u8, TestStruct, false>::from_bytes(&data);
        assert!(matches!(result, Err(ZeroCopyError::UnalignedPointer)),);
    }
    {
        let mut data =
            vec![0; ZeroCopySlice::<u8, TestStruct, true>::required_size_for_capacity(1)];
        data[0] = 1;
        ZeroCopySlice::<u8, TestStruct, true>::from_bytes(&data).unwrap();
    }
}

/// Succeeds because derives Unaligned
#[test]
fn test_unaligned_success() {
    #[derive(
        Copy,
        Clone,
        PartialEq,
        Debug,
        Default,
        Immutable,
        FromBytes,
        KnownLayout,
        IntoBytes,
        Unaligned,
    )]
    #[repr(C)]
    struct TestStruct {
        a: U32,
        aa: U32,
        b: U64,
        c: U16,
        cc: U16,
        ccc: U32,
        d: [u8; 32],
    }
    let mut data = vec![0; ZeroCopySlice::<u8, TestStruct>::required_size_for_capacity(1)];
    data[0] = 1;
    let usize_len: usize = u64::try_from(1).unwrap() as usize;
    let slice = ZeroCopySlice::<u8, TestStruct, false>::from_bytes(&data).unwrap();
    assert_eq!(slice.len(), usize_len);
}

#[test]
fn test_zero_copy_u8_struct_vec() {
    #[derive(
        Copy, Clone, PartialEq, Debug, Default, Immutable, FromBytes, IntoBytes, KnownLayout,
    )]
    struct TestStruct {
        a: [u8; 32],
    }
    impl Distribution<TestStruct> for Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestStruct {
            TestStruct { a: rng.gen() }
        }
    }

    test_zero_copy_slice_new::<u8, TestStruct>(u8::MAX, u8::MAX.to_le_bytes().to_vec());
    test_zero_copy_slice_new::<u16, TestStruct>(
        u8::MAX as u16,
        (u8::MAX as u16).to_le_bytes().to_vec(),
    );
    test_zero_copy_slice_new::<u32, TestStruct>(
        u8::MAX as u32,
        (u8::MAX as u32).to_le_bytes().to_vec(),
    );
    test_zero_copy_slice_new::<u64, TestStruct>(
        u8::MAX as u64,
        (u8::MAX as u64).to_le_bytes().to_vec(),
    );
}

#[test]
fn test_empty() {
    let length = 0;
    let data = vec![0; ZeroCopySlice::<u8, u8>::required_size_for_capacity(length)];
    let zero_copy_slice = ZeroCopySlice::<u8, u8>::from_bytes(&data).unwrap();
    let usize_len: usize = length as usize;

    assert_eq!(zero_copy_slice.len(), usize_len);
    assert!(zero_copy_slice.is_empty());
    assert_eq!(zero_copy_slice.first(), None);
    assert_eq!(zero_copy_slice.last(), None);
    assert_eq!(zero_copy_slice.get(0), None);
    assert_eq!(zero_copy_slice.as_slice(), &[]);
    assert_eq!(zero_copy_slice.iter().cloned().collect::<Vec<u8>>(), vec![]);
    assert_eq!(zero_copy_slice.to_vec(), vec![]);
}

#[test]
#[should_panic = "out of bounds"]
fn test_index_out_of_bounds_zero() {
    let length = 1;
    let data = vec![0; ZeroCopySlice::<u8, u8>::required_size_for_capacity(length)];
    let zero_copy_slice = ZeroCopySlice::<u8, u8>::from_bytes(&data).unwrap();
    let _ = zero_copy_slice[length as usize];
}

#[test]
#[should_panic = "out of bounds"]
fn test_index_out_of_bounds_non_zero() {
    let length = 10;
    let mut data = vec![0; ZeroCopySlice::<u8, u8>::required_size_for_capacity(length)];
    data[0] = 10;
    let zero_copy_slice = ZeroCopySlice::<u8, u8>::from_bytes(&data).unwrap();
    let _ = zero_copy_slice[length as usize];
}

/// Test that metadata size is aligned to T.
#[test]
fn test_metadata_size() {
    assert_eq!(ZeroCopySlice::<u8, u8>::metadata_size(), 1);
    assert_eq!(ZeroCopySlice::<u16, u8>::metadata_size(), 2);
    assert_eq!(ZeroCopySlice::<u32, u8>::metadata_size(), 4);
    assert_eq!(ZeroCopySlice::<u64, u8>::metadata_size(), 8);

    assert_eq!(ZeroCopySlice::<u8, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySlice::<u16, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySlice::<u32, u16>::metadata_size(), 4);
    assert_eq!(ZeroCopySlice::<u64, u16>::metadata_size(), 8);

    assert_eq!(ZeroCopySlice::<u8, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySlice::<u16, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySlice::<u32, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySlice::<u64, u32>::metadata_size(), 8);

    assert_eq!(ZeroCopySlice::<u8, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySlice::<u16, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySlice::<u32, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySlice::<u64, u64>::metadata_size(), 8);
}

#[test]
fn test_data_size() {
    let length = 64;
    assert_eq!(ZeroCopySlice::<u8, u8>::data_size(length), 64);
    assert_eq!(ZeroCopySlice::<u16, u8>::data_size(length as u16), 64);
    assert_eq!(ZeroCopySlice::<u32, u8>::data_size(length as u32), 64);
    assert_eq!(ZeroCopySlice::<u64, u8>::data_size(length as u64), 64);

    assert_eq!(ZeroCopySlice::<u8, u16>::data_size(length), 128);
    assert_eq!(ZeroCopySlice::<u16, u16>::data_size(length as u16), 128);
    assert_eq!(ZeroCopySlice::<u32, u16>::data_size(length as u32), 128);
    assert_eq!(ZeroCopySlice::<u64, u16>::data_size(length as u64), 128);

    assert_eq!(ZeroCopySlice::<u8, u32>::data_size(length), 256);
    assert_eq!(ZeroCopySlice::<u16, u32>::data_size(length as u16), 256);
    assert_eq!(ZeroCopySlice::<u32, u32>::data_size(length as u32), 256);
    assert_eq!(ZeroCopySlice::<u64, u32>::data_size(length as u64), 256);

    assert_eq!(ZeroCopySlice::<u8, u64>::data_size(length), 512);
    assert_eq!(ZeroCopySlice::<u16, u64>::data_size(length as u16), 512);
    assert_eq!(ZeroCopySlice::<u32, u64>::data_size(length as u32), 512);
    assert_eq!(ZeroCopySlice::<u64, u64>::data_size(length as u64), 512);
}

#[test]
fn test_required_size() {
    let length = 64;
    assert_eq!(
        ZeroCopySlice::<u8, u8>::required_size_for_capacity(length),
        64 + 1
    );
    assert_eq!(
        ZeroCopySlice::<u16, u8>::required_size_for_capacity(length as u16),
        64 + 2
    );
    assert_eq!(
        ZeroCopySlice::<u32, u8>::required_size_for_capacity(length as u32),
        64 + 4
    );
    assert_eq!(
        ZeroCopySlice::<u64, u8>::required_size_for_capacity(length as u64),
        64 + 8
    );

    assert_eq!(
        ZeroCopySlice::<u8, u16>::required_size_for_capacity(length),
        128 + 2
    );
    assert_eq!(
        ZeroCopySlice::<u16, u16>::required_size_for_capacity(length as u16),
        128 + 2
    );
    assert_eq!(
        ZeroCopySlice::<u32, u16>::required_size_for_capacity(length as u32),
        128 + 4
    );
    assert_eq!(
        ZeroCopySlice::<u64, u16>::required_size_for_capacity(length as u64),
        128 + 8
    );

    assert_eq!(
        ZeroCopySlice::<u8, u32>::required_size_for_capacity(length),
        256 + 4
    );
    assert_eq!(
        ZeroCopySlice::<u16, u32>::required_size_for_capacity(length as u16),
        256 + 4
    );
    assert_eq!(
        ZeroCopySlice::<u32, u32>::required_size_for_capacity(length as u32),
        256 + 4
    );
    assert_eq!(
        ZeroCopySlice::<u64, u32>::required_size_for_capacity(length as u64),
        256 + 8
    );

    assert_eq!(
        ZeroCopySlice::<u8, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySlice::<u16, u64>::required_size_for_capacity(length as u16),
        512 + 8
    );
    assert_eq!(
        ZeroCopySlice::<u32, u64>::required_size_for_capacity(length as u32),
        512 + 8
    );
    assert_eq!(
        ZeroCopySlice::<u64, u64>::required_size_for_capacity(length as u64),
        512 + 8
    );
}

#[test]
fn test_copy_from_slice_and_try_into_array() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceU64::<U32>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopySliceMutU64::<U32>::new(capacity, &mut data).unwrap();

    for i in &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
        vec[*i] = U32::new(*i as u32);
    }
    let vec = ZeroCopySliceU64::<U32>::from_bytes(&data).unwrap();

    let arr = vec.try_into_array().unwrap();
    assert_eq!(arr, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

    assert!(matches!(
        vec.try_into_array::<15>(),
        Err(ZeroCopyError::ArraySize(_, _))
    ));
    assert!(matches!(
        vec.try_into_array::<17>(),
        Err(ZeroCopyError::ArraySize(_, _))
    ));
}

#[test]
fn test_failing_from_bytes_at() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceU64::<U32>::required_size_for_capacity(capacity)];
    data[0..size_of::<u64>()].copy_from_slice(&capacity.to_le_bytes());
    let vec = ZeroCopySliceU64::<U32>::from_bytes_at(&data[..7]);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
    let vec = ZeroCopySliceU64::<U32>::from_bytes_at(&data[..9]);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
}

#[test]
fn test_data_as_ptr_and_data_as_mut_ptr() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];
    {
        let required_size = ZeroCopySlice::<u64, u32, true>::required_size_for_capacity(length);
        buffer[0..size_of::<u64>()].copy_from_slice(&length.to_le_bytes());
        assert!(buffer.len() >= required_size);
        let (mut slice, _) = ZeroCopySliceMutU64::<u32>::from_bytes_at(&mut buffer)
            .expect("Failed to create ZeroCopySlice");
        slice.as_mut_slice().copy_from_slice(&values);
    }
    {
        let (slice, _) = ZeroCopySliceU64::<u32>::from_bytes_at(&buffer)
            .expect("Failed to create ZeroCopySlice");
        let data_ptr = slice.data_as_ptr();
        unsafe {
            for (i, value) in values.iter().enumerate() {
                assert_eq!(*data_ptr.add(i), *value);
            }
        }
        assert_eq!(slice.as_slice(), &values);
    }
}
use light_zero_copy::traits::ZeroCopyAt;
#[test]
fn test_zero_copy_at() {
    let mut buffer = vec![0u8; 32];
    let length = 4;
    let values = [1u32, 2, 3, 4];
    {
        let (mut slice, _) = ZeroCopySliceMut::<u32, u32, true>::new_at(length, &mut buffer)
            .expect("Failed to create ZeroCopySlice");
        slice.as_mut_slice().copy_from_slice(&values);
    }
    let (slice, _) = ZeroCopySliceBorsh::<u32>::from_bytes_at(&buffer).unwrap();
    let (zero_slice, _) = ZeroCopySliceBorsh::<u32>::zero_copy_at(&buffer).unwrap();
    assert_eq!(slice.as_slice(), &values);
    assert_eq!(zero_slice.as_slice(), slice.as_slice());
}

#[test]
fn test_into_iter_immutable() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];
    {
        let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
            .expect("Failed to create ZeroCopySlice");
        slice.as_mut_slice().copy_from_slice(&values);
    }
    let (slice, _) = ZeroCopySliceU64::<u32>::from_bytes_at(&buffer).unwrap();
    let mut iter = slice.into_iter();
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), Some(&4));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_partial_eq() {
    let mut buffer1 = vec![0u8; 32];
    let mut buffer2 = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];
    {
        let (mut slice1, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer1)
            .expect("Failed to create ZeroCopySlice");
        let (mut slice2, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer2)
            .expect("Failed to create ZeroCopySlice");
        slice1.as_mut_slice().copy_from_slice(&values);
        slice2.as_mut_slice().copy_from_slice(&values);
    }
    let (slice1, _) = ZeroCopySlice::<u64, u32, true>::from_bytes_at(&buffer1)
        .expect("Failed to create ZeroCopySlice");
    let (slice2, _) = ZeroCopySlice::<u64, u32, true>::from_bytes_at(&buffer2)
        .expect("Failed to create ZeroCopySlice");

    assert_eq!(slice1, slice2);
}

#[test]
fn test_debug_fmt() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];

    {
        let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
            .expect("Failed to create ZeroCopySlice");
        slice.as_mut_slice().copy_from_slice(&values);
    }
    let (slice, _) = ZeroCopySlice::<u64, u32, true>::from_bytes_at(&buffer)
        .expect("Failed to create ZeroCopySlice");
    assert_eq!(format!("{:?}", slice), "[1, 2, 3, 4]");
}
