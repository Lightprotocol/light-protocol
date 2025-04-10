#![cfg(feature = "std")]
use core::fmt::Debug;

use light_zero_copy::{
    add_padding,
    errors::ZeroCopyError,
    slice_mut::{ZeroCopySliceMut, ZeroCopySliceMutU64},
};
use rand::{distributions::Standard, prelude::*};
use zerocopy::{
    little_endian::{U16, U32, U64},
    FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned,
};

fn test_zero_copy_slice_mut_new<LEN, T>(length: LEN)
where
    LEN: Copy + FromBytes + Immutable + KnownLayout + IntoBytes,
    T: Copy + Clone + PartialEq + Debug + Default + FromBytes + Immutable + KnownLayout + IntoBytes,
    u64: From<LEN>,
    Standard: Distribution<T>,
{
    let mut rng = thread_rng();
    let mut data = vec![0; ZeroCopySliceMut::<LEN, T>::required_size_for_capacity(length)];
    ZeroCopySliceMut::<LEN, T>::new(length, &mut data).unwrap();
    let usize_len: usize = u64::from(length) as usize;

    // Test from_bytes with a zeroed slice
    {
        let reference_vec = vec![T::default(); usize_len];
        println!("data len {}", data.len());
        let slice = ZeroCopySliceMut::<LEN, T>::from_bytes(&mut data).unwrap();
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

    let length_usize: usize = usize_len;
    let mut reference_vec = vec![T::default(); length_usize];

    // Fill the slice completely and verify properties
    {
        let mut slice = ZeroCopySliceMut::<LEN, T>::from_bytes(&mut data).unwrap();

        for i in 0..length_usize {
            let element = rng.gen();
            slice.as_mut_slice()[i] = element;
            reference_vec[i] = element;

            // 1. Check slice length
            assert_eq!(slice.len(), length_usize);
            assert!(!slice.is_empty());
            // 2. Validate elements by index
            assert_eq!(slice[i], element);
            // 3. Validate get & get_mut
            assert_eq!(slice.get(i), Some(&element));
            assert_eq!(slice.get_mut(i), Some(&mut reference_vec[i]));

            // 4. Validate first & last
            assert_eq!(slice.first(), Some(&reference_vec[0]));
            assert_eq!(slice.first_mut(), Some(&mut reference_vec[0]));
            assert_eq!(slice.last(), reference_vec.last());
            assert_eq!(slice.last_mut(), reference_vec.last_mut());
            // 5. Validate as_slice
            assert_eq!(slice.as_slice(), reference_vec.as_slice());
            assert_eq!(slice.as_mut_slice(), reference_vec.as_mut_slice());
            assert_eq!(slice.to_vec(), reference_vec);
        }

        // 6. iterate over elements
        for (index, element) in slice.iter().enumerate() {
            assert_eq!(element, &reference_vec[index]);
        }

        // 7. Mutate elements via iter_mut
        for (index, element) in slice.iter_mut().enumerate() {
            let new_element = rng.gen();
            *element = new_element;
            reference_vec[index] = new_element;
        }

        // Ensure the slice matches the reference after mutation
        assert_eq!(slice.as_slice(), reference_vec.as_slice());
    }
}

#[test]
fn test_zero_copy_vec() {
    test_zero_copy_slice_mut_new::<u8, u32>(u8::MAX);
    println!("test_zero_copy_vec_with_capacity::<u8>()");
    test_zero_copy_slice_mut_new::<U16, u32>((u8::MAX as u16).into());
    println!("test_zero_copy_vec_with_capacity::<u16>()");
    test_zero_copy_slice_mut_new::<U32, u32>((u8::MAX as u32).into());
    println!("test_zero_copy_vec_with_capacity::<u32>()");
    test_zero_copy_slice_mut_new::<U64, u32>((u8::MAX as u64).into());
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

    test_zero_copy_slice_mut_new::<u8, TestStruct>(u8::MAX);
    test_zero_copy_slice_mut_new::<U16, TestStruct>((u8::MAX as u16).into());
    test_zero_copy_slice_mut_new::<U32, TestStruct>((u8::MAX as u32).into());
    test_zero_copy_slice_mut_new::<U64, TestStruct>((u8::MAX as u64).into());
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
            vec![0; ZeroCopySliceMut::<u8, TestStruct, false>::required_size_for_capacity(1)];
        let result = ZeroCopySliceMut::<u8, TestStruct, false>::new(1, &mut data);
        assert!(matches!(result, Err(ZeroCopyError::UnalignedPointer)),);
    }
    {
        let mut data =
            vec![0; ZeroCopySliceMut::<u8, TestStruct, true>::required_size_for_capacity(1)];
        ZeroCopySliceMut::<u8, TestStruct, true>::new(1, &mut data).unwrap();
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
    let mut data = vec![0; ZeroCopySliceMut::<u8, TestStruct>::required_size_for_capacity(1)];
    ZeroCopySliceMut::<u8, TestStruct>::new(1, &mut data).unwrap();
    let usize_len: usize = u64::try_from(1).unwrap() as usize;
    let slice = ZeroCopySliceMut::<u8, TestStruct, false>::from_bytes(&mut data).unwrap();
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

    test_zero_copy_slice_mut_new::<u8, TestStruct>(u8::MAX);
    test_zero_copy_slice_mut_new::<u16, TestStruct>(u8::MAX as u16);
    test_zero_copy_slice_mut_new::<u32, TestStruct>(u8::MAX as u32);
    test_zero_copy_slice_mut_new::<u64, TestStruct>(u8::MAX as u64);
}

#[test]
fn test_empty() {
    let length = 0;
    let mut data = vec![0; ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length)];
    let mut zero_copy_slice = ZeroCopySliceMut::<u8, u8>::new(length, &mut data).unwrap();
    let usize_len: usize = length as usize;

    assert_eq!(zero_copy_slice.len(), usize_len);
    assert!(zero_copy_slice.is_empty());
    assert_eq!(zero_copy_slice.first(), None);
    assert_eq!(zero_copy_slice.last(), None);
    assert_eq!(zero_copy_slice.get(0), None);
    assert_eq!(zero_copy_slice.get_mut(0), None);
    assert_eq!(zero_copy_slice.as_slice(), &[]);
    assert_eq!(zero_copy_slice.iter().cloned().collect::<Vec<u8>>(), vec![]);
    assert!(
        zero_copy_slice.iter_mut().next().is_none(),
        "Expected no elements"
    );
    assert_eq!(zero_copy_slice.as_mut_slice(), &[]);

    assert_eq!(zero_copy_slice.to_vec(), vec![]);
}

#[test]
#[should_panic = "out of bounds"]
fn test_index_out_of_bounds() {
    let length = 1;
    let mut data = vec![0; ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length)];
    let zero_copy_slice = ZeroCopySliceMut::<u8, u8>::new(length, &mut data).unwrap();
    let _ = zero_copy_slice[length as usize];
}

/// Test that metadata size is aligned to T.
#[test]
fn test_metadata_size() {
    assert_eq!(ZeroCopySliceMut::<u8, u8>::metadata_size(), 1);
    assert_eq!(ZeroCopySliceMut::<u16, u8>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u32, u8>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u8>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u16, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u32, u16>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u16>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u16, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u32, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u32>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u16, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u32, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u64, u64>::metadata_size(), 8);
}

#[test]
fn test_data_size() {
    let length = 64;
    assert_eq!(ZeroCopySliceMut::<u8, u8>::data_size(length), 64);
    assert_eq!(ZeroCopySliceMut::<u16, u8>::data_size(length as u16), 64);
    assert_eq!(ZeroCopySliceMut::<u32, u8>::data_size(length as u32), 64);
    assert_eq!(ZeroCopySliceMut::<u64, u8>::data_size(length as u64), 64);

    assert_eq!(ZeroCopySliceMut::<u8, u16>::data_size(length), 128);
    assert_eq!(ZeroCopySliceMut::<u16, u16>::data_size(length as u16), 128);
    assert_eq!(ZeroCopySliceMut::<u32, u16>::data_size(length as u32), 128);
    assert_eq!(ZeroCopySliceMut::<u64, u16>::data_size(length as u64), 128);

    assert_eq!(ZeroCopySliceMut::<u8, u32>::data_size(length), 256);
    assert_eq!(ZeroCopySliceMut::<u16, u32>::data_size(length as u16), 256);
    assert_eq!(ZeroCopySliceMut::<u32, u32>::data_size(length as u32), 256);
    assert_eq!(ZeroCopySliceMut::<u64, u32>::data_size(length as u64), 256);

    assert_eq!(ZeroCopySliceMut::<u8, u64>::data_size(length), 512);
    assert_eq!(ZeroCopySliceMut::<u16, u64>::data_size(length as u16), 512);
    assert_eq!(ZeroCopySliceMut::<u32, u64>::data_size(length as u32), 512);
    assert_eq!(ZeroCopySliceMut::<u64, u64>::data_size(length as u64), 512);
}

#[test]
fn test_required_size() {
    let length = 64;
    assert_eq!(
        ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length),
        64 + 1
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u8>::required_size_for_capacity(length as u16),
        64 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u8>::required_size_for_capacity(length as u32),
        64 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u8>::required_size_for_capacity(length as u64),
        64 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u16>::required_size_for_capacity(length),
        128 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u16>::required_size_for_capacity(length as u16),
        128 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u16>::required_size_for_capacity(length as u32),
        128 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u16>::required_size_for_capacity(length as u64),
        128 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u32>::required_size_for_capacity(length),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u32>::required_size_for_capacity(length as u16),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u32>::required_size_for_capacity(length as u32),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u32>::required_size_for_capacity(length as u64),
        256 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u64>::required_size_for_capacity(length as u16),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u64>::required_size_for_capacity(length as u32),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u64>::required_size_for_capacity(length as u64),
        512 + 8
    );
}

#[test]
fn test_copy_from_slice_and_try_into_array() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutU64::<U32>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopySliceMutU64::<U32>::new(capacity, &mut data).unwrap();

    for i in &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15] {
        vec[*i] = U32::new(*i as u32);
    }

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
fn test_failing_new() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutU64::<U32>::required_size_for_capacity(capacity) - 1];
    let vec = ZeroCopySliceMutU64::<U32>::new(capacity, &mut data);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));

    let mut data = vec![1; ZeroCopySliceMutU64::<U32>::required_size_for_capacity(capacity)];
    let vec = ZeroCopySliceMutU64::<U32>::new(capacity, &mut data);
    assert!(matches!(vec, Err(ZeroCopyError::MemoryNotZeroed)));
}

#[test]
fn test_failing_from_bytes_at() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutU64::<U32>::required_size_for_capacity(capacity)];
    ZeroCopySliceMutU64::<U32>::new(capacity, &mut data).unwrap();
    let vec = ZeroCopySliceMutU64::<U32>::from_bytes_at(&mut data[..7]);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
    let vec = ZeroCopySliceMutU64::<U32>::from_bytes_at(&mut data[..9]);
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
    let required_size = ZeroCopySliceMut::<u64, u32, true>::required_size_for_capacity(length);
    assert!(buffer.len() >= required_size);
    let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
        .expect("Failed to create ZeroCopySliceMut");
    slice.as_mut_slice().copy_from_slice(&values);
    let data_ptr = slice.data_as_ptr();
    unsafe {
        for (i, value) in values.iter().enumerate() {
            assert_eq!(*data_ptr.add(i), *value);
        }
    }
    let data_mut_ptr = slice.data_as_mut_ptr();
    unsafe {
        for i in 0..length as usize {
            *data_mut_ptr.add(i) += 1;
        }
    }
    let expected_values = [2u32, 3, 4, 5];
    assert_eq!(slice.as_slice(), &expected_values);
}

#[test]
fn test_into_iter_immutable() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];
    let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
        .expect("Failed to create ZeroCopySliceMut");
    slice.as_mut_slice().copy_from_slice(&values);
    let mut iter = slice.into_iter();
    assert_eq!(iter.next(), Some(&1));
    assert_eq!(iter.next(), Some(&2));
    assert_eq!(iter.next(), Some(&3));
    assert_eq!(iter.next(), Some(&4));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_into_iter_mutable() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];
    let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
        .expect("Failed to create ZeroCopySliceMut");
    slice.as_mut_slice().copy_from_slice(&values);
    for x in &mut slice {
        *x += 10;
    }
    assert_eq!(slice.as_slice(), &[11, 12, 13, 14]);
}

#[test]
fn test_partial_eq() {
    let mut buffer1 = vec![0u8; 32];
    let mut buffer2 = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];

    let (mut slice1, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer1)
        .expect("Failed to create ZeroCopySliceMut");
    let (mut slice2, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer2)
        .expect("Failed to create ZeroCopySliceMut");

    slice1.as_mut_slice().copy_from_slice(&values);
    slice2.as_mut_slice().copy_from_slice(&values);

    assert_eq!(slice1, slice2);

    slice2.as_mut_slice()[0] = 10;
    assert_ne!(slice1, slice2);
}

#[test]
fn test_debug_fmt() {
    let mut buffer = vec![0u8; 32];
    let length: u64 = 4;
    let values = [1u32, 2, 3, 4];

    let (mut slice, _) = ZeroCopySliceMut::<u64, u32, true>::new_at(length, &mut buffer)
        .expect("Failed to create ZeroCopySliceMut");
    slice.as_mut_slice().copy_from_slice(&values);

    assert_eq!(format!("{:?}", slice), "[1, 2, 3, 4]");
}
