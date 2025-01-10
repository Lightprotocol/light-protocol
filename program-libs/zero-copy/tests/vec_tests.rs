use std::fmt::Debug;

use light_zero_copy::{
    add_padding,
    errors::ZeroCopyError,
    slice_mut::ZeroCopyTraits,
    vec::{ZeroCopyVec, ZeroCopyVecU64},
};
use rand::{
    distributions::{Distribution, Standard},
    thread_rng, Rng,
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Generates a random value, excluding the values provided
/// in `exclude`.
fn gen_exclude<N, T>(rng: &mut N, exclude: &[T]) -> T
where
    N: Rng,
    T: PartialEq,
    Standard: Distribution<T>,
{
    loop {
        // This utility is supposed to be used only in unit tests. This `clone`
        // is harmless and necessary (can't pass a reference to range, it has
        // to be moved).
        let sample = rng.gen();
        if !exclude.contains(&sample) {
            return sample;
        }
    }
}

#[test]
fn test_zero_copy_vec() {
    test_zero_copy_vec_new::<u8, u32>(u8::MAX);
    println!("test_zero_copy_vec_with_capacity::<u8>()");
    test_zero_copy_vec_new::<u16, u32>(u8::MAX as u16);
    println!("test_zero_copy_vec_with_capacity::<u16>()");
    test_zero_copy_vec_new::<u32, u32>(u8::MAX as u32);
    println!("test_zero_copy_vec_with_capacity::<u32>()");
    test_zero_copy_vec_new::<u64, u32>(u8::MAX as u64);
    println!("test_zero_copy_vec_with_capacity::<u64>()");
    test_zero_copy_vec_new::<u64, u32>(10000);
}

#[test]
fn test_zero_copy_u64_struct_vec() {
    #[derive(
        Copy, Clone, PartialEq, Debug, Default, Immutable, FromBytes, KnownLayout, IntoBytes,
    )]
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

    test_zero_copy_vec_new::<u8, TestStruct>(u8::MAX);
    test_zero_copy_vec_new::<u16, TestStruct>(u8::MAX as u16);
    test_zero_copy_vec_new::<u32, TestStruct>(u8::MAX as u32);
    test_zero_copy_vec_new::<u64, TestStruct>(u8::MAX as u64);
}

#[test]
fn test_zero_copy_u8_struct_vec() {
    #[derive(
        Copy, Clone, PartialEq, Debug, Default, Immutable, FromBytes, KnownLayout, IntoBytes,
    )]
    struct TestStruct {
        a: [u8; 32],
    }
    impl Distribution<TestStruct> for Standard {
        fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> TestStruct {
            TestStruct { a: rng.gen() }
        }
    }

    test_zero_copy_vec_new::<u8, TestStruct>(u8::MAX);
    test_zero_copy_vec_new::<u16, TestStruct>(u8::MAX as u16);
    test_zero_copy_vec_new::<u32, TestStruct>(u8::MAX as u32);
    test_zero_copy_vec_new::<u64, TestStruct>(u8::MAX as u64);
}

fn test_zero_copy_vec_new<CAPACITY, T>(capacity: CAPACITY)
where
    CAPACITY: ZeroCopyTraits + Debug + PartialEq,
    T: ZeroCopyTraits + Debug + PartialEq + Copy,
    u64: From<CAPACITY> + TryInto<CAPACITY>,
    Standard: Distribution<T>,
{
    let mut rng = thread_rng();
    let mut data = vec![0; ZeroCopyVec::<CAPACITY, T>::required_size_for_capacity(capacity)];
    println!("data len: {}", data.len());
    println!("capacity: {:?}", capacity);
    // new
    {
        let zero_copy_vec = ZeroCopyVec::<CAPACITY, T>::new(capacity, &mut data).unwrap();
        assert_eq!(zero_copy_vec.capacity() as u64, capacity.into());
        assert_eq!(zero_copy_vec.len(), 0);
        assert!(zero_copy_vec.is_empty());
    }
    println!("new {:?}", data[0..8].to_vec());
    // empty from bytes
    {
        let reference_vec = vec![];
        let vec = ZeroCopyVec::<CAPACITY, T>::from_bytes(&mut data).unwrap();
        assert_empty_vec(capacity, reference_vec, vec);
        {
            let data = data.clone();
            let mut metadata_size = size_of::<CAPACITY>();
            let length = data[0..metadata_size].to_vec();

            assert_eq!(length, vec![0; metadata_size]);

            let padding_start = metadata_size.clone();
            add_padding::<CAPACITY, T>(&mut metadata_size);
            let padding_end = metadata_size;
            let data = data[padding_start..padding_end].to_vec();
            // Padding should be zeroed
            assert_eq!(data, vec![0; padding_end - padding_start]);
        }
    }
    let capacity_usize: usize = (u64::try_from(capacity)
        .map_err(|_| ZeroCopyError::InvalidConversion)
        .unwrap()) as usize;
    let mut reference_vec = vec![];
    // fill vector completely and assert:
    {
        // 1. vector is not empty
        // 2. vector length is correct
        // 3. vector capacity is correct
        // 4. vector elements can be accessed by index
        // 5. vector elements can be accessed by get
        // 6. vector elements can be accessed by get_mut
        // 7. vector last element can be accessed
        // 8. vector last element can be accessed mutably
        // 9. vector first element can be accessed
        // 10. vector first element can be accessed mutably
        // 11. vector as_slice returns correct slice
        // 12. vector as_mut_slice returns correct slice
        // 13. vector to_vec returns correct vector
        // 14. (iter) iterating over vector returns correct elements
        // 15. (iter_mut) iterating over vector mutably returns correct elements
        for i in 0..capacity_usize {
            let mut vec = ZeroCopyVec::<CAPACITY, T>::from_bytes(&mut data).unwrap();

            let element = rng.gen();
            vec.push(element).unwrap();
            reference_vec.push(element);
            // 1. vector is not empty
            assert!(!vec.is_empty());
            // 2. vector length is correct
            assert_eq!(vec.len(), i + 1);

            // 3. vector capacity is correct
            assert_eq!(vec.capacity(), capacity_usize);
            // 4. vector elements can be accessed by index
            assert_eq!(vec[i], element);
            // 5. vector elements can be accessed by get
            assert_eq!(vec.get(i), Some(&element));
            // 6. vector elements can be accessed by get_mut
            assert_eq!(vec.get_mut(i), Some(&mut reference_vec[i]));
            // 7. vector last element can be accessed
            assert_eq!(vec.last(), Some(&element));
            // 8. vector last element can be accessed mutably
            assert_eq!(vec.last_mut(), Some(&mut reference_vec[i]));
            // 9. vector first element can be accessed
            assert_eq!(vec.first(), Some(&reference_vec[0]));
            // 10. vector first element can be accessed mutably
            assert_eq!(vec.first_mut(), Some(&mut reference_vec[0]));
            // 11. vector as_slice returns correct slice
            assert_eq!(vec.as_slice(), reference_vec.as_slice());
            assert_ne!(&vec.as_slice()[1..], reference_vec.as_slice());
            // 12. vector as_mut_slice returns correct slice
            assert_eq!(vec.as_mut_slice(), reference_vec.as_mut_slice());
            assert_ne!(&vec.as_mut_slice()[1..], reference_vec.as_mut_slice());
            // 13. vector to_vec returns correct vector
            assert_eq!(vec.to_vec(), reference_vec);
            assert_ne!(vec.to_vec()[1..].to_vec(), reference_vec);
            // 14. (iter) iterating over vector returns correct elements
            for (index, element) in vec.iter().enumerate() {
                assert_eq!(*element, reference_vec[index]);
            }
            // 15. (iter_mut) iterating over vector mutably returns correct elements
            for (index, element) in vec.iter_mut().enumerate() {
                assert_eq!(*element, reference_vec[index]);
                let new_element = gen_exclude(&mut rng, &[*element]);
                *element = new_element;
                assert_ne!(element, &reference_vec[index]);
                reference_vec[index] = *element;
            }
            {
                let cloned_data: Vec<u8> = data.clone();
                let mut metadata_size = size_of::<CAPACITY>();
                let mut length = cloned_data[0..metadata_size].to_vec();
                while length.len() < 8 {
                    length.push(0);
                }
                assert_eq!(length, ((i as u64 + 1).to_ne_bytes().as_ref().to_vec()));

                let padding_start = metadata_size.clone();
                add_padding::<CAPACITY, T>(&mut metadata_size);
                let padding_end = metadata_size;
                let cloned_data = cloned_data[padding_start..padding_end].to_vec();
                // Padding should be zeroed
                assert_eq!(cloned_data, vec![0; padding_end - padding_start]);
            }
        }
    }
    // full vec from bytes
    {
        let mut vec = ZeroCopyVec::<CAPACITY, T>::from_bytes(&mut data).unwrap();
        assert_full_vec(
            capacity,
            &mut rng,
            capacity_usize,
            &mut reference_vec,
            &mut vec,
        );
        // Failing push into full vec
        let result = vec.push(rng.gen());
        assert_eq!(result, Err(ZeroCopyError::Full));
    }
    // clear
    {
        let mut vec = ZeroCopyVec::<CAPACITY, T>::from_bytes(&mut data).unwrap();
        vec.clear();
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.capacity(), capacity_usize);
        assert!(vec.is_empty());
        assert_ne!(vec.as_slice(), reference_vec.as_slice());
        assert_ne!(vec.as_mut_slice(), reference_vec.as_mut_slice());
        assert_ne!(vec.to_vec(), reference_vec);
        reference_vec.clear();
        assert_empty_vec(capacity, reference_vec, vec);
        {
            let data = data.clone();
            let mut metadata_size = size_of::<CAPACITY>();
            let mut length = data[0..metadata_size].to_vec();
            let ref_length: CAPACITY = 0u64
                .try_into()
                .map_err(|_| ZeroCopyError::InvalidConversion)
                .unwrap(); //;(0).to_usize().unwrap();
            while length.len() < 8 {
                length.push(0);
            }
            assert_eq!(
                length,
                (u64::try_from(ref_length)
                    .map_err(|_| ZeroCopyError::InvalidConversion)
                    .unwrap())
                .to_le_bytes()
                .as_ref()
                .to_vec()
            );

            let padding_start = metadata_size;
            add_padding::<CAPACITY, T>(&mut metadata_size);
            let padding_end = metadata_size;
            let data = data[padding_start..padding_end].to_vec();
            // Padding should be zeroed
            assert_eq!(data, vec![0; padding_end - padding_start]);
        }
    }
}

fn assert_full_vec<CAPACITY, T>(
    capacity: CAPACITY,
    rng: &mut rand::prelude::ThreadRng,
    capacity_usize: usize,
    reference_vec: &mut Vec<T>,
    vec: &mut ZeroCopyVec<CAPACITY, T>,
) where
    CAPACITY: ZeroCopyTraits + Debug + PartialEq,
    T: ZeroCopyTraits + Debug + PartialEq,
    Standard: Distribution<T>,
    u64: From<CAPACITY> + TryInto<CAPACITY>,
{
    // 1. vector capacity is correct
    assert_eq!(vec.capacity() as u64, capacity.try_into().unwrap());
    // 2. vector length is correct
    assert_eq!(vec.len(), capacity_usize);
    // 3. vector is not empty
    assert!(!vec.is_empty());
    // 4. vector as slice returns correct slice
    assert_eq!(vec.as_slice(), reference_vec.as_slice());
    assert_ne!(&vec.as_slice()[1..], reference_vec.as_slice());
    // 5. vector as_mut_slice returns correct slice
    assert_eq!(vec.as_mut_slice(), reference_vec.as_mut_slice());
    assert_ne!(&vec.as_mut_slice()[1..], reference_vec.as_mut_slice());
    // 6. vector to_vec returns correct vector
    assert_eq!(vec.to_vec(), *reference_vec);
    assert_ne!(vec.to_vec()[1..].to_vec(), *reference_vec);
    // 7. (iter) iterating over vector returns correct elements
    for (index, element) in vec.iter().enumerate() {
        assert_eq!(*element, reference_vec[index]);
    }
    // 8. (iter_mut) iterating over vector mutably returns correct elements
    for (index, element) in vec.iter_mut().enumerate() {
        assert_eq!(*element, reference_vec[index]);
        *element = rng.gen();
        assert_ne!(element, &reference_vec[index]);
        reference_vec[index] = *element;
    }
}

fn assert_empty_vec<CAPACITY, T>(
    capacity: CAPACITY,
    mut reference_vec: Vec<T>,
    mut vec: ZeroCopyVec<CAPACITY, T>,
) where
    CAPACITY: ZeroCopyTraits + Debug + PartialEq,
    T: ZeroCopyTraits + Debug + PartialEq + Copy,
    u64: From<CAPACITY> + TryInto<CAPACITY>,
{
    // 1. vector capacity is correct
    assert_eq!(vec.capacity() as u64, capacity.into());
    // 2. vector length is correct
    assert_eq!(vec.len(), 0);
    // 3. vector is empty
    assert!(vec.is_empty());
    // 4. vector elements can be accessed by get
    assert_eq!(vec.get(0), None);
    // 5. vector elements can be accessed by get_mut
    assert_eq!(vec.get_mut(0), None);
    // 6. vector last element can be accessed
    assert_eq!(vec.last(), None);
    // 7. vector last element can be accessed mutably
    assert_eq!(vec.last_mut(), None);
    // 8. vector first element can be accessed
    assert_eq!(vec.first(), None);
    // 9. vector first element can be accessed mutably
    assert_eq!(vec.first_mut(), None);
    // 10. vector as_slice returns correct slice
    assert_eq!(vec.as_slice(), reference_vec.as_slice());
    // 11. vector as_mut_slice returns correct slice
    assert_eq!(vec.as_mut_slice(), reference_vec.as_mut_slice());
    // 12. vector to_vec returns correct vector
    assert_eq!(vec.to_vec(), reference_vec);
    // 13. (iter) iterating over vector returns correct elements
    for (index, element) in vec.iter().enumerate() {
        panic!(
            "Should not iterate over empty vector: index={}, element={:?}",
            index, element
        );
    }
    // 14. (iter_mut) iterating over vector mutably returns correct elements
    for (index, element) in vec.iter_mut().enumerate() {
        panic!(
            "Should not iterate over empty vector: index={}, element={:?}",
            index, element
        );
    }
}

#[test]
fn test_zero_copy_vec_to_array() {
    let capacity = 16;
    let mut data = vec![0; ZeroCopyVecU64::<u32>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyVecU64::<u32>::new(capacity as u64, &mut data).unwrap();
    vec.extend_from_slice(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

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
fn test_zero_copy_vec_into_iter() {
    let capacity = 1000;
    let mut data = vec![0; ZeroCopyVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyVecU64::<usize>::new(capacity as u64, &mut data).unwrap();

    for i in 0..1000 {
        vec.push(i).unwrap();
    }

    for (i, element) in vec.into_iter().enumerate() {
        assert_eq!(*element, i);
    }
}

#[test]
fn test_new_at_and_from_bytes_at_multiple() {
    let mut account_data = vec![0u8; ZeroCopyVecU64::<u64>::required_size_for_capacity(4) * 2];
    // test new_at & fill vectors
    {
        let (mut vec, remaining_bytes) =
            ZeroCopyVecU64::<u64>::new_at(4, &mut account_data).unwrap();
        for i in 0..4 {
            vec.push(i as u64).unwrap();
        }
        assert_eq!(
            remaining_bytes.len(),
            ZeroCopyVecU64::<u64>::required_size_for_capacity(4)
        );

        let (mut vec, remaining_bytes) = ZeroCopyVecU64::<u64>::new_at(4, remaining_bytes).unwrap();
        for i in 4..8 {
            vec.push(i as u64).unwrap();
        }
        assert_eq!(remaining_bytes.len(), 0);
    }
    // test from_bytes_at_multiple
    {
        let (deserialized_vecs, remaining_bytes) =
            ZeroCopyVecU64::<u64>::from_bytes_at_multiple(2, &mut account_data)
                .expect("Failed to deserialize multiple ZeroCopyCyclicVecs");

        assert_eq!(deserialized_vecs.len(), 2);
        for (i, deserialized_vec) in deserialized_vecs.iter().enumerate() {
            for (j, element) in deserialized_vec.iter().enumerate() {
                assert_eq!(*element, (i * 4 + j) as u64);
            }
        }
        assert_eq!(0, remaining_bytes.len());
    }
}

#[test]
fn test_init_multiple_pass() {
    let mut account_data = vec![0u8; 128];
    let capacity = 4usize;
    let (mut initialized_vecs, remaining_bytes) =
        ZeroCopyVecU64::<u64>::new_at_multiple(2, capacity as u64, &mut account_data).unwrap();
    assert_eq!(
        remaining_bytes.len(),
        128 - ZeroCopyVecU64::<u64>::required_size_for_capacity(capacity as u64) * 2
    );
    assert_eq!(initialized_vecs.len(), 2);
    assert_eq!(initialized_vecs[0].capacity(), capacity);
    assert_eq!(initialized_vecs[1].capacity(), capacity);
    assert_eq!(initialized_vecs[0].len(), 0);
    assert_eq!(initialized_vecs[1].len(), 0);
    let mut reference_vecs = [vec![], vec![]];
    for i in 0..capacity {
        for (j, vec) in initialized_vecs.iter_mut().enumerate() {
            assert!(vec.get(i).is_none());
            vec.push(i as u64).unwrap();
            reference_vecs[j].push(i as u64);
            assert_eq!(*vec.get(i).unwrap(), i as u64);
            assert!(vec.len() == i + 1);
        }
    }
    for (i, vec) in initialized_vecs.iter_mut().enumerate() {
        let mut rng = thread_rng();
        assert_full_vec(
            capacity as u64,
            &mut rng,
            capacity as usize,
            &mut reference_vecs[i],
            vec,
        );
    }
}

#[test]
fn test_metadata_size() {
    assert_eq!(ZeroCopyVec::<u8, u8>::metadata_size(), 1);
    assert_eq!(ZeroCopyVec::<u16, u8>::metadata_size(), 2);
    assert_eq!(ZeroCopyVec::<u32, u8>::metadata_size(), 4);
    assert_eq!(ZeroCopyVec::<u64, u8>::metadata_size(), 8);

    assert_eq!(ZeroCopyVec::<u8, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopyVec::<u16, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopyVec::<u32, u16>::metadata_size(), 4);
    assert_eq!(ZeroCopyVec::<u64, u16>::metadata_size(), 8);

    assert_eq!(ZeroCopyVec::<u8, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyVec::<u16, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyVec::<u32, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyVec::<u64, u32>::metadata_size(), 8);

    assert_eq!(ZeroCopyVec::<u8, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyVec::<u16, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyVec::<u32, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyVec::<u64, u64>::metadata_size(), 8);
}

#[test]
fn test_data_size() {
    assert_eq!(ZeroCopyVec::<u8, u8>::data_size(64), 65);
}

#[test]
fn test_required_size() {
    // length + capacity + data
    assert_eq!(
        ZeroCopyVec::<u8, u8>::required_size_for_capacity(64),
        1 + 1 + 64
    );
    // length + capacity + data
    assert_eq!(
        ZeroCopyVec::<u64, u64>::required_size_for_capacity(64),
        8 + 8 + 8 * 64
    );
}

#[test]
fn test_partial_eq() {
    let mut account_data = vec![0u8; ZeroCopyVecU64::<u64>::required_size_for_capacity(5)];
    let mut vec = ZeroCopyVecU64::<u64>::new(5, &mut account_data).unwrap();
    for i in 0..4 {
        vec.push(i as u64).unwrap();
    }

    let mut account_data = vec![0u8; ZeroCopyVecU64::<u64>::required_size_for_capacity(5)];
    let mut vec2 = ZeroCopyVecU64::<u64>::new(5, &mut account_data).unwrap();
    for i in 0..4 {
        vec2.push(i as u64).unwrap();
    }

    assert_eq!(vec, vec2);

    vec2.push(5).unwrap();
    assert_ne!(vec, vec2);
    vec.push(5).unwrap();
    assert_eq!(vec, vec2);
    vec.clear();
    assert_ne!(vec, vec2);
}
