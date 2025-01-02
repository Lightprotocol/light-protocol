use core::fmt::Debug;
use std::{convert::TryInto, ops::Add};

use light_zero_copy::{
    add_padding,
    errors::ZeroCopyError,
    slice_mut::{ZeroCopySliceMut, ZeroCopySliceMutUsize},
};
use rand::{distributions::Standard, prelude::*};

fn test_zero_copy_slice_mut_new<LEN, T>(length: LEN)
where
    LEN: Debug
        + Copy
        + Clone
        + Add<Output = LEN>
        + TryInto<usize>
        + TryFrom<usize>
        + PartialOrd
        + num_traits::ToBytes,
    <LEN as TryFrom<usize>>::Error: std::fmt::Debug,
    <LEN as TryInto<usize>>::Error: std::fmt::Debug,
    T: Copy + Clone + PartialEq + Debug + Default,
    Standard: Distribution<T>,
{
    let mut rng = thread_rng();
    let mut data =
        vec![0; ZeroCopySliceMut::<LEN, T>::required_size_for_capacity(length.try_into().unwrap())];
    ZeroCopySliceMut::<LEN, T>::new(length, &mut data).unwrap();

    // Test from_bytes with a zeroed slice
    {
        let reference_vec = vec![T::default(); length.try_into().unwrap()];
        let slice = ZeroCopySliceMut::<LEN, T>::from_bytes(&mut data).unwrap();
        // 1. Validate length
        assert_eq!(slice.len(), reference_vec.len());
        {
            let data = data.clone();
            let mut metadata_size = size_of::<LEN>();
            let length_bytes = data[0..metadata_size].to_vec();
            assert_eq!(length_bytes, length.to_ne_bytes().as_ref().to_vec());

            let padding_start = metadata_size.clone();
            add_padding::<LEN, T>(&mut metadata_size);
            let padding_end = metadata_size;
            let data = data[padding_start..padding_end].to_vec();
            // Padding should be zeroed
            assert_eq!(data, vec![0; padding_end - padding_start]);
        }

        // 2. Validate slice content
        assert_eq!(slice.as_slice(), reference_vec.as_slice());

        // 3. Validate is not_empty
        assert!(!slice.is_empty());

        // 4. Validate accessors return None
        assert_eq!(T::default(), *slice.first().unwrap());
        assert_eq!(T::default(), *slice.last().unwrap());
    }

    let length_usize: usize = length.try_into().unwrap();
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
    test_zero_copy_slice_mut_new::<u16, u32>(u8::MAX as u16);
    println!("test_zero_copy_vec_with_capacity::<u16>()");
    test_zero_copy_slice_mut_new::<u32, u32>(u8::MAX as u32);
    println!("test_zero_copy_vec_with_capacity::<u32>()");
    test_zero_copy_slice_mut_new::<u64, u32>(u8::MAX as u64);
    println!("test_zero_copy_vec_with_capacity::<u64>()");
    test_zero_copy_slice_mut_new::<usize, u32>(10000 as usize);
}

#[test]
fn test_zero_copy_u64_struct_vec() {
    #[derive(Copy, Clone, PartialEq, Debug, Default)]
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
    test_zero_copy_slice_mut_new::<u16, TestStruct>(u8::MAX as u16);
    test_zero_copy_slice_mut_new::<u32, TestStruct>(u8::MAX as u32);
    test_zero_copy_slice_mut_new::<u64, TestStruct>(u8::MAX as u64);
    test_zero_copy_slice_mut_new::<usize, TestStruct>(u8::MAX as usize);
}

#[test]
fn test_zero_copy_u8_struct_vec() {
    #[derive(Copy, Clone, PartialEq, Debug, Default)]
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
    test_zero_copy_slice_mut_new::<usize, TestStruct>(u8::MAX as usize);
}

#[test]
fn test_empty() {
    let length = 0;
    let mut data = vec![0; ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length)];
    let mut zero_copy_slice =
        ZeroCopySliceMut::<u8, u8>::new(u8::try_from(length).unwrap(), &mut data).unwrap();
    assert_eq!(zero_copy_slice.len(), length.try_into().unwrap());
    assert!(zero_copy_slice.is_empty());
    assert_eq!(zero_copy_slice.first(), None);
    assert_eq!(zero_copy_slice.last(), None);
    assert_eq!(zero_copy_slice.get(0), None);
    assert_eq!(zero_copy_slice.get_mut(0), None);
    assert_eq!(zero_copy_slice.as_slice(), &[]);
    assert_eq!(zero_copy_slice.iter().cloned().collect::<Vec<u8>>(), vec![]);
    for element in zero_copy_slice.iter_mut() {
        panic!("Expected no elements, found {:?}", element);
    }
    assert_eq!(zero_copy_slice.as_mut_slice(), &[]);
    assert_eq!(zero_copy_slice.to_vec(), vec![]);
}

#[test]
#[should_panic = "out of bounds"]
fn test_index_out_of_bounds() {
    let length = 1;
    let mut data = vec![0; ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length)];
    let zero_copy_slice =
        ZeroCopySliceMut::<u8, u8>::new(u8::try_from(length).unwrap(), &mut data).unwrap();
    zero_copy_slice[length];
}

/// Test that metadata size is aligned to T.
#[test]
fn test_metadata_size() {
    assert_eq!(ZeroCopySliceMut::<u8, u8>::metadata_size(), 1);
    assert_eq!(ZeroCopySliceMut::<u16, u8>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u32, u8>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u8>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<usize, u8>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u16, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopySliceMut::<u32, u16>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u16>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<usize, u16>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u16, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u32, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopySliceMut::<u64, u32>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<usize, u32>::metadata_size(), 8);

    assert_eq!(ZeroCopySliceMut::<u8, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u16, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u32, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<u64, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopySliceMut::<usize, u64>::metadata_size(), 8);
}

#[test]
fn test_data_size() {
    let length = 64;
    assert_eq!(ZeroCopySliceMut::<u8, u8>::data_size(length), 64);
    assert_eq!(ZeroCopySliceMut::<u16, u8>::data_size(length as u16), 64);
    assert_eq!(ZeroCopySliceMut::<u32, u8>::data_size(length as u32), 64);
    assert_eq!(ZeroCopySliceMut::<u64, u8>::data_size(length as u64), 64);
    assert_eq!(
        ZeroCopySliceMut::<usize, u8>::data_size(length as usize),
        64
    );

    assert_eq!(ZeroCopySliceMut::<u8, u16>::data_size(length), 128);
    assert_eq!(ZeroCopySliceMut::<u16, u16>::data_size(length as u16), 128);
    assert_eq!(ZeroCopySliceMut::<u32, u16>::data_size(length as u32), 128);
    assert_eq!(ZeroCopySliceMut::<u64, u16>::data_size(length as u64), 128);
    assert_eq!(
        ZeroCopySliceMut::<usize, u16>::data_size(length as usize),
        128
    );

    assert_eq!(ZeroCopySliceMut::<u8, u32>::data_size(length), 256);
    assert_eq!(ZeroCopySliceMut::<u16, u32>::data_size(length as u16), 256);
    assert_eq!(ZeroCopySliceMut::<u32, u32>::data_size(length as u32), 256);
    assert_eq!(ZeroCopySliceMut::<u64, u32>::data_size(length as u64), 256);
    assert_eq!(
        ZeroCopySliceMut::<usize, u32>::data_size(length as usize),
        256
    );

    assert_eq!(ZeroCopySliceMut::<u8, u64>::data_size(length), 512);
    assert_eq!(ZeroCopySliceMut::<u16, u64>::data_size(length as u16), 512);
    assert_eq!(ZeroCopySliceMut::<u32, u64>::data_size(length as u32), 512);
    assert_eq!(ZeroCopySliceMut::<u64, u64>::data_size(length as u64), 512);
    assert_eq!(
        ZeroCopySliceMut::<usize, u64>::data_size(length as usize),
        512
    );
}

#[test]
fn test_required_size() {
    let length = 64;
    assert_eq!(
        ZeroCopySliceMut::<u8, u8>::required_size_for_capacity(length),
        64 + 1
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u8>::required_size_for_capacity(length),
        64 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u8>::required_size_for_capacity(length),
        64 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u8>::required_size_for_capacity(length),
        64 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<usize, u8>::required_size_for_capacity(length),
        64 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u16>::required_size_for_capacity(length),
        128 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u16>::required_size_for_capacity(length),
        128 + 2
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u16>::required_size_for_capacity(length),
        128 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u16>::required_size_for_capacity(length),
        128 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<usize, u16>::required_size_for_capacity(length),
        128 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u32>::required_size_for_capacity(length),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u32>::required_size_for_capacity(length),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u32>::required_size_for_capacity(length),
        256 + 4
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u32>::required_size_for_capacity(length),
        256 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<usize, u32>::required_size_for_capacity(length),
        256 + 8
    );

    assert_eq!(
        ZeroCopySliceMut::<u8, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u16, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u32, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<u64, u64>::required_size_for_capacity(length),
        512 + 8
    );
    assert_eq!(
        ZeroCopySliceMut::<usize, u64>::required_size_for_capacity(length),
        512 + 8
    );
}

#[test]
fn test_new_at_and_from_bytes_at_multiple() {
    let mut account_data =
        vec![0u8; ZeroCopySliceMutUsize::<u64>::required_size_for_capacity(4) * 2];
    // test new_at & fill vectors
    {
        let mut offset = 0;
        let mut vec =
            ZeroCopySliceMutUsize::<u64>::new_at(4, &mut account_data, &mut offset).unwrap();
        for i in 0..4 {
            vec[i] = i as u64;
        }
        assert_eq!(
            offset,
            ZeroCopySliceMutUsize::<u64>::required_size_for_capacity(4)
        );

        let mut vec =
            ZeroCopySliceMutUsize::<u64>::new_at(4, &mut account_data, &mut offset).unwrap();
        for i in 0..4 {
            vec[i] = i as u64 + 4;
        }
        assert_eq!(
            offset,
            ZeroCopySliceMutUsize::<u64>::required_size_for_capacity(4) * 2
        );
    }
    // test from_bytes_at_multiple
    {
        let mut offset = 0;
        let deserialized_vecs =
            ZeroCopySliceMutUsize::<u64>::from_bytes_at_multiple(2, &mut account_data, &mut offset)
                .expect("Failed to deserialize multiple ZeroCopyCyclicVecs");

        assert_eq!(deserialized_vecs.len(), 2);
        for (i, deserialized_vec) in deserialized_vecs.iter().enumerate() {
            for (j, element) in deserialized_vec.iter().enumerate() {
                assert_eq!(*element, (i * 4 + j) as u64);
            }
        }
        assert_eq!(
            offset,
            ZeroCopySliceMutUsize::<u64>::required_size_for_capacity(4) * 2
        );
    }
}

#[test]
fn test_new_at_multiple() {
    let mut account_data = vec![0u8; 128];
    let mut offset = 0;
    let capacity = 4;
    let mut reference_vecs = vec![vec![], vec![]];

    {
        let mut initialized_vecs = ZeroCopySliceMutUsize::<u64>::new_at_multiple(
            2,
            capacity,
            &mut account_data,
            &mut offset,
        )
        .unwrap();
        assert_eq!(
            offset,
            ZeroCopySliceMutUsize::<u64>::required_size_for_capacity(capacity) * 2
        );
        assert_eq!(initialized_vecs.len(), 2);
        assert_eq!(initialized_vecs[0].len(), capacity);
        assert_eq!(initialized_vecs[1].len(), capacity);
        for i in 0..capacity {
            for (j, vec) in initialized_vecs.iter_mut().enumerate() {
                assert_eq!(vec.get(i), Some(&0));
                vec[i] = i as u64;
                reference_vecs[j].push(i as u64);
                assert_eq!(*vec.get(i).unwrap(), i as u64);
                assert_eq!(vec.len(), capacity);
            }
        }
    }
    {
        let vecs =
            ZeroCopySliceMutUsize::<u64>::from_bytes_at_multiple(2, &mut account_data, &mut 0)
                .unwrap();
        for (i, vec) in vecs.iter().enumerate() {
            assert_eq!(vec.to_vec(), reference_vecs[i]);
        }
    }
}

#[test]
fn test_copy_from_slice_and_try_into_array() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutUsize::<u32>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopySliceMutUsize::<u32>::new(capacity, &mut data).unwrap();
    vec.copy_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

    let arr: [u32; 16] = vec.try_into_array().unwrap();
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
fn test_at_ptr() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutUsize::<u32>::required_size_for_capacity(capacity)];
    let data_mut_ptr = data[ZeroCopySliceMutUsize::<u32>::metadata_size()..].as_mut_ptr();
    let data_ptr = data[ZeroCopySliceMutUsize::<u32>::metadata_size()..].as_ptr();
    let mut vec = ZeroCopySliceMutUsize::<u32>::new(capacity, &mut data).unwrap();
    assert_eq!(data_ptr as *const u32, vec.data_as_ptr());
    assert_eq!(data_mut_ptr as *mut u32, vec.data_as_mut_ptr());
}

#[test]
fn test_failing_new() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutUsize::<u32>::required_size_for_capacity(capacity) - 1];
    let vec = ZeroCopySliceMutUsize::<u32>::new(capacity, &mut data);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
}

#[test]
fn test_failing_from_bytes_at() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopySliceMutUsize::<u32>::required_size_for_capacity(capacity)];
    ZeroCopySliceMutUsize::<u32>::new(capacity, &mut data).unwrap();
    let vec = ZeroCopySliceMutUsize::<u32>::from_bytes_at(&mut data[..7], &mut 0);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
    let vec = ZeroCopySliceMutUsize::<u32>::from_bytes_at(&mut data[..9], &mut 0);
    assert!(matches!(
        vec,
        Err(ZeroCopyError::InsufficientMemoryAllocated(_, _))
    ));
}
