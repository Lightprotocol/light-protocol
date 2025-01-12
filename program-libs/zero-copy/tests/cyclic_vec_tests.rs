use light_zero_copy::{
    cyclic_vec::{ZeroCopyCyclicVec, ZeroCopyCyclicVecU64},
    errors::ZeroCopyError,
};
use rand::{thread_rng, Rng};

#[test]
fn test_cyclic_bounded_vec_with_capacity() {
    for capacity in 0..1024 {
        let mut data = vec![0; ZeroCopyCyclicVecU64::<u32>::required_size_for_capacity(capacity)];
        let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<u32>::new(capacity, &mut data).unwrap();

        assert_eq!(cyclic_bounded_vec.capacity(), capacity as usize);
        assert_eq!(cyclic_bounded_vec.len(), 0);
        assert_eq!(cyclic_bounded_vec.first_index(), 0);
        assert_eq!(cyclic_bounded_vec.last_index(), 0);
        assert_eq!(cyclic_bounded_vec.as_slice(), &[]);
        assert_eq!(cyclic_bounded_vec.as_mut_slice(), &mut []);
    }
}

#[test]
fn test_cyclic_bounded_vec_is_empty() {
    let mut rng = thread_rng();
    let capacity = 1000;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<u32>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<u32>::new(capacity, &mut data).unwrap();

    assert!(vec.is_empty());
    let mut ref_vec = Vec::new();
    for _ in 0..1000 {
        let element = rng.gen();
        vec.push(element);
        ref_vec.push(element);
        assert!(!vec.is_empty());
    }
    assert_eq!(vec.as_slice(), ref_vec.as_slice());
    assert_eq!(vec.as_mut_slice(), ref_vec.as_mut_slice());
    let array: [u32; 1000] = vec.try_into_array().unwrap();
    assert_eq!(array, <[u32; 1000]>::try_from(ref_vec).unwrap());
}

#[test]
fn test_cyclic_bounded_vec_get() {
    let capacity = 1000;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..1000 {
        vec.push(i);
    }

    for i in 0..1000 {
        assert_eq!(vec.get(i), Some(&i));
    }
    for i in 1000..10_000 {
        assert!(vec.get(i).is_none());
    }
}

#[test]
fn test_cyclic_bounded_vec_get_mut() {
    let capacity = 1000;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..2000 {
        vec.push(i);
    }

    for i in 0..1000 {
        let element = vec.get_mut(i).unwrap();
        assert_eq!(*element, 1000 + i);
        *element = i * 2;
    }
    for i in 0..1000 {
        assert_eq!(vec.get_mut(i), Some(&mut (i * 2)));
    }
    for i in 1000..10_000 {
        assert!(vec.get_mut(i).is_none());
    }
}

#[test]
fn test_cyclic_bounded_vec_first() {
    let capacity = 500;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    assert!(vec.first().is_none());

    for i in 0..1000 {
        vec.push(i);
        assert_eq!(vec.last(), Some(&i));
        assert_eq!(vec.last_mut(), Some(&mut i.clone()));
        assert_eq!(vec.first(), Some(&((i).saturating_sub(499))));
        assert_eq!(vec.first_mut(), Some(&mut ((i).saturating_sub(499))));
    }
}

#[test]
fn test_cyclic_bounded_vec_last() {
    let mut rng = thread_rng();
    let capacity = 500;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    assert!(vec.last().is_none());

    for _ in 0..1000 {
        let element = rng.gen();
        vec.push(element);

        assert_eq!(vec.last(), Some(&element));
    }
}

#[test]
fn test_cyclic_bounded_vec_last_mut() {
    let mut rng = thread_rng();
    let capacity = 500;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    assert!(vec.last_mut().is_none());

    for i in 0..1000 {
        let element_old = rng.gen();
        vec.push(element_old);
        println!("last index {:?}", vec.last_index());
        println!("i: {}", i);
        let element_ref = vec.last_mut().unwrap();
        assert_eq!(*element_ref, element_old);

        // Assign a new value.
        let element_new = rng.gen();
        *element_ref = element_new;

        // Assert that it took the effect.
        let element_ref = vec.last_mut().unwrap();
        assert_eq!(*element_ref, element_new);
    }
}

#[test]
fn test_cyclic_bounded_vec_manual() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    // Fill up the cyclic vector.
    //
    // ```
    //        ^                    $
    // index [0, 1, 2, 3, 4, 5, 6, 7]
    // value [0, 1, 2, 3, 4, 5, 6, 7]
    // ```
    //
    // * `^` - first element
    // * `$` - last element
    for i in 0..8 {
        cyclic_bounded_vec.push(i);
    }
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 7);
    assert_eq!(
        cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
        &[&0, &1, &2, &3, &4, &5, &6, &7]
    );

    // Overwrite half of values.
    //
    // ```
    //                   $  ^
    // index [0, 1,  2,  3, 4, 5, 6, 7]
    // value [8, 9, 10, 11, 4, 5, 6, 7]
    // ```
    //
    // * `^` - first element
    // * `$` - last element
    for i in 0..4 {
        cyclic_bounded_vec.push(i + 8);
    }
    assert_eq!(cyclic_bounded_vec.first_index(), 4);
    assert_eq!(cyclic_bounded_vec.last_index(), 3);
    assert_eq!(
        cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
        &[&4, &5, &6, &7, &8, &9, &10, &11]
    );

    // Overwrite even more.
    //
    // ```
    //                           $  ^
    // index [0, 1,  2,  3,  4,  5, 6, 7]
    // value [8, 9, 10, 11, 12, 13, 6, 7]
    // ```
    //
    // * `^` - first element
    // * `$` - last element
    for i in 0..2 {
        cyclic_bounded_vec.push(i + 12);
    }
    assert_eq!(cyclic_bounded_vec.first_index(), 6);
    assert_eq!(cyclic_bounded_vec.last_index(), 5);
    assert_eq!(
        cyclic_bounded_vec
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .as_slice(),
        &[6, 7, 8, 9, 10, 11, 12, 13]
    );

    // Overwrite all values from the first loop.
    //
    // ```
    //        ^                          $
    // index [0, 1,  2,  3,  4,  5,  6,  7]
    // value [8, 9, 10, 11, 12, 13, 14, 15]
    // ```
    //
    // * `^` - first element
    // * `$` - last element
    for i in 0..2 {
        cyclic_bounded_vec.push(i + 14);
    }
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 7);
    assert_eq!(
        cyclic_bounded_vec
            .iter()
            .cloned()
            .collect::<Vec<_>>()
            .as_slice(),
        &[8, 9, 10, 11, 12, 13, 14, 15]
    );
}

/// Iteration on a vector with one element.
///
/// ```
///        ^$
/// index [0]
/// value [0]
/// ```
///
/// * `^` - first element
/// * `$` - last element
/// * `#` - visited elements
///
/// Length: 1
/// Capacity: 8
/// First index: 0
/// Last index: 0
///
/// Start iteration from: 0
///
/// Should iterate over one element.
#[test]
fn test_cyclic_bounded_vec_iter_one_element() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    cyclic_bounded_vec.push(0);

    assert_eq!(cyclic_bounded_vec.len(), 1);
    assert_eq!(cyclic_bounded_vec.capacity(), 8);
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 0);

    let elements = cyclic_bounded_vec.iter().collect::<Vec<_>>();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements.as_slice(), &[&0]);

    let elements = cyclic_bounded_vec.iter_from(0).unwrap().collect::<Vec<_>>();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements.as_slice(), &[&0]);
}

/// Iteration without reset in a vector which is not full.
///
/// ```
///              #  #  #  #
///        ^              $
/// index [0, 1, 2, 3, 4, 5]
/// value [0, 1, 2, 3, 4, 5]
/// ```
///
/// * `^` - first element
/// * `$` - last element
/// * `#` - visited elements
///
/// Length: 6
/// Capacity: 8
/// First index: 0
/// Last index: 5
///
/// Start iteration from: 2
///
/// Should iterate over elements from 2 to 5, with 4 iterations.
#[test]
fn test_cyclic_bounded_vec_iter_from_without_reset_not_full_6_8_4() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..6 {
        cyclic_bounded_vec.push(i);
    }

    assert_eq!(cyclic_bounded_vec.len(), 6);
    assert_eq!(cyclic_bounded_vec.capacity(), 8);
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 5);

    let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
    assert_eq!(elements.len(), 4);
    assert_eq!(elements.as_slice(), &[&2, &3, &4, &5]);
}
/// Iteration without reset in a vector which is full.
///
/// ```
///              #  #  #
///        ^           $
/// index [0, 1, 2, 3, 4]
/// value [0, 1, 2, 3, 4]
/// ```
///
/// * `^` - first element
/// * `$` - last element
/// * `#` - visited elements
///
/// Length: 5
/// Capacity: 5
/// First index: 0
/// Last index: 4
///
/// Start iteration from: 2
///
/// Should iterate over elements 2..4 - 3 iterations.
#[test]
fn test_cyclic_bounded_vec_iter_from_without_reset_not_full_5_5_4() {
    let capacity = 5;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..5 {
        cyclic_bounded_vec.push(i);
    }

    assert_eq!(cyclic_bounded_vec.len(), 5);
    assert_eq!(cyclic_bounded_vec.capacity(), 5);
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 4);

    let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
    assert_eq!(elements.len(), 3);
    assert_eq!(elements.as_slice(), &[&2, &3, &4]);
}

/// Iteration without reset in a vector which is full.
///
/// ```
///              #  #  #  #  #  #
///        ^                    $
/// index [0, 1, 2, 3, 4, 5, 6, 7]
/// value [0, 1, 2, 3, 4, 5, 6, 7]
/// ```
///
/// * `^` - first element
/// * `$` - last element
/// * `#` - visited elements
///
/// Length: 8
/// Capacity: 8
/// First index: 0
/// Last index: 7
///
/// Start iteration from: 2
///
/// Should iterate over elements 2..7 - 6 iterations.
#[test]
fn test_cyclic_bounded_vec_iter_from_without_reset_full_8_8_6() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..8 {
        cyclic_bounded_vec.push(i);
    }

    assert_eq!(cyclic_bounded_vec.len(), 8);
    assert_eq!(cyclic_bounded_vec.capacity(), 8);
    assert_eq!(cyclic_bounded_vec.first_index(), 0);
    assert_eq!(cyclic_bounded_vec.last_index(), 7);

    let elements = cyclic_bounded_vec.iter_from(2).unwrap().collect::<Vec<_>>();
    assert_eq!(elements.len(), 6);
    assert_eq!(elements.as_slice(), &[&2, &3, &4, &5, &6, &7]);
}

/// Iteration with reset.
///
/// Insert elements over capacity, so the vector resets and starts
/// overwriting elements from the start - 12 elements into a vector with
/// capacity 8.
///
/// The resulting data structure looks like:
///
/// ```
///        #  #   #   #        #  #
///                   $  ^
/// index [0, 1,  2,  3, 4, 5, 6, 7]
/// value [8, 9, 10, 11, 4, 5, 6, 7]
/// ```
///
/// * `^` - first element
/// * `$` - last element
/// * `#` - visited elements
///
/// Length: 8
/// Capacity: 8
/// First: 4
/// Last: 3
///
/// Start iteration from: 6
///
/// Should iterate over elements 6..7 and 8..11 - 6 iterations.
#[test]
fn test_cyclic_bounded_vec_iter_from_reset() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..12 {
        cyclic_bounded_vec.push(i);
    }

    assert_eq!(cyclic_bounded_vec.len(), 8);
    assert_eq!(cyclic_bounded_vec.capacity(), 8);
    assert_eq!(cyclic_bounded_vec.first_index(), 4);
    assert_eq!(cyclic_bounded_vec.last_index(), 3);

    let elements = cyclic_bounded_vec.iter_from(6).unwrap().collect::<Vec<_>>();
    assert_eq!(elements.len(), 6);
    assert_eq!(elements.as_slice(), &[&6, &7, &8, &9, &10, &11]);
}

#[test]
fn test_cyclic_bounded_vec_iter_from_out_of_bounds_not_full() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..4 {
        cyclic_bounded_vec.push(i);
    }

    // Try `start` values in bounds.
    for i in 0..4 {
        let elements = cyclic_bounded_vec.iter_from(i).unwrap().collect::<Vec<_>>();
        assert_eq!(elements.len(), 4 - i);
        let expected = (i..4).collect::<Vec<_>>();
        // Just to coerce it to have references...
        let expected = expected.iter().collect::<Vec<_>>();
        assert_eq!(elements.as_slice(), expected.as_slice());
    }

    // Try `start` values out of bounds.
    for i in 4..1000 {
        let elements = cyclic_bounded_vec.iter_from(i);
        assert!(matches!(elements, Err(ZeroCopyError::IterFromOutOfBounds)));
    }
}

#[test]
fn test_cyclic_bounded_vec_iter_from_out_of_bounds_full() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..12 {
        cyclic_bounded_vec.push(i);
    }

    // Try different `start` values which are out of bounds.
    for start in 8..1000 {
        let elements = cyclic_bounded_vec.iter_from(start);
        assert!(matches!(elements, Err(ZeroCopyError::IterFromOutOfBounds)));
    }
}

#[test]
fn test_cyclic_bounded_vec_iter_from_out_of_bounds_iter_from() {
    let capacity = 8;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..8 {
        assert!(matches!(
            cyclic_bounded_vec.iter_from(i),
            Err(ZeroCopyError::IterFromOutOfBounds)
        ));
        cyclic_bounded_vec.push(i);
    }
}

#[test]
fn test_cyclic_bounded_vec_overwrite() {
    let capacity = 64u64;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut cyclic_bounded_vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    for i in 0..256 {
        cyclic_bounded_vec.push(i);
    }

    assert_eq!(cyclic_bounded_vec.len(), 64);
    assert_eq!(cyclic_bounded_vec.capacity(), 64);
    assert_eq!(
        cyclic_bounded_vec.iter().collect::<Vec<_>>().as_slice(),
        &[
            &192, &193, &194, &195, &196, &197, &198, &199, &200, &201, &202, &203, &204, &205,
            &206, &207, &208, &209, &210, &211, &212, &213, &214, &215, &216, &217, &218, &219,
            &220, &221, &222, &223, &224, &225, &226, &227, &228, &229, &230, &231, &232, &233,
            &234, &235, &236, &237, &238, &239, &240, &241, &242, &243, &244, &245, &246, &247,
            &248, &249, &250, &251, &252, &253, &254, &255
        ]
    );
}

#[test]
fn test_clear_pass() {
    let capacity = 5u64;
    let mut data = vec![0; ZeroCopyCyclicVecU64::<usize>::required_size_for_capacity(capacity)];
    let mut vec = ZeroCopyCyclicVecU64::<usize>::new(capacity, &mut data).unwrap();

    vec.push(1);
    vec.push(2);
    vec.clear();
    assert_eq!(vec.len(), 0);
    assert!(vec.get(0).is_none());
    assert!(vec.get(1).is_none());
}

#[test]
fn test_deserialize_pass() {
    let mut account_data = vec![0u8; ZeroCopyCyclicVecU64::<u64>::required_size_for_capacity(4)];

    // Initialize data with valid ZeroCopyCyclicVecU64 metadata and elements
    ZeroCopyCyclicVecU64::<u64>::new(4, &mut account_data).unwrap();

    // Deserialize the ZeroCopyCyclicVecU64
    let deserialized_vec = ZeroCopyCyclicVecU64::<u64>::from_bytes(&mut account_data)
        .expect("Failed to deserialize ZeroCopyCyclicVecU64");

    assert_eq!(deserialized_vec.capacity(), 4);
    assert_eq!(deserialized_vec.len(), 0);
}

#[test]
fn test_deserialize_multiple_pass() {
    let mut account_data =
        vec![0u8; ZeroCopyCyclicVecU64::<u64>::required_size_for_capacity(4) * 2];

    {
        // Initialize data for multiple ZeroCopyCyclicVecs
        let (_, account_data) = ZeroCopyCyclicVecU64::<u64>::new_at(4, &mut account_data).unwrap();
        ZeroCopyCyclicVecU64::<u64>::new_at(4, account_data).unwrap();
    }
    // Deserialize multiple ZeroCopyCyclicVecs
    let (deserialized_vecs, _) =
        ZeroCopyCyclicVecU64::<u64>::from_bytes_at_multiple(2, &mut account_data)
            .expect("Failed to deserialize multiple ZeroCopyCyclicVecs");

    assert_eq!(deserialized_vecs.len(), 2);
    for vec in deserialized_vecs.iter() {
        assert_eq!(vec.capacity(), 4);
        assert_eq!(vec.len(), 0);
        assert_eq!(vec.to_vec(), vec![]);
    }
}

#[test]
fn test_init_pass() {
    let mut account_data = vec![0u8; 64];

    // Initialize a ZeroCopyCyclicVecU64 with capacity 4
    let (mut vec, _) = ZeroCopyCyclicVecU64::<u64>::new_at(4, &mut account_data).unwrap();
    assert_eq!(vec.capacity(), 4);
    assert_eq!(vec.len(), 0);
    for i in 0..4 {
        assert!(vec.get(i).is_none());
        vec.push(i as u64);
        assert_eq!(*vec.get(i).unwrap(), i as u64);
        assert!(vec.len() == i + 1);
    }
}

#[test]
fn test_init_multiple_pass() {
    let mut account_data = vec![0u8; 128];
    let (mut initialized_vecs, _) =
        ZeroCopyCyclicVecU64::<u64>::new_at_multiple(2, 4, &mut account_data).unwrap();

    assert_eq!(initialized_vecs.len(), 2);
    assert_eq!(initialized_vecs[0].capacity(), 4);
    assert_eq!(initialized_vecs[1].capacity(), 4);
    assert_eq!(initialized_vecs[0].len(), 0);
    assert_eq!(initialized_vecs[1].len(), 0);
    for i in 0..4 {
        for vec in initialized_vecs.iter_mut() {
            assert!(vec.get(i).is_none());
            vec.push(i as u64);
            assert_eq!(*vec.get(i).unwrap(), i as u64);
            assert!(vec.len() == i + 1);
        }
    }
}

#[test]
fn test_metadata_size() {
    assert_eq!(ZeroCopyCyclicVec::<u8, u8>::metadata_size(), 1);
    assert_eq!(ZeroCopyCyclicVec::<u16, u8>::metadata_size(), 2);
    assert_eq!(ZeroCopyCyclicVec::<u32, u8>::metadata_size(), 4);
    assert_eq!(ZeroCopyCyclicVec::<u64, u8>::metadata_size(), 8);

    assert_eq!(ZeroCopyCyclicVec::<u8, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopyCyclicVec::<u16, u16>::metadata_size(), 2);
    assert_eq!(ZeroCopyCyclicVec::<u32, u16>::metadata_size(), 4);
    assert_eq!(ZeroCopyCyclicVec::<u64, u16>::metadata_size(), 8);

    assert_eq!(ZeroCopyCyclicVec::<u8, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyCyclicVec::<u16, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyCyclicVec::<u32, u32>::metadata_size(), 4);
    assert_eq!(ZeroCopyCyclicVec::<u64, u32>::metadata_size(), 8);

    assert_eq!(ZeroCopyCyclicVec::<u8, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyCyclicVec::<u16, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyCyclicVec::<u32, u64>::metadata_size(), 8);
    assert_eq!(ZeroCopyCyclicVec::<u64, u64>::metadata_size(), 8);
}

#[test]
fn test_data_size() {
    assert_eq!(ZeroCopyCyclicVec::<u8, u8>::data_size(64), 66);
}

#[test]
fn test_required_size() {
    // current index + length + capacity + data
    assert_eq!(
        ZeroCopyCyclicVec::<u8, u8>::required_size_for_capacity(64),
        1 + 1 + 1 + 64
    );
    // current index + length + capacity + data
    assert_eq!(
        ZeroCopyCyclicVec::<u64, u64>::required_size_for_capacity(64),
        8 + 8 + 8 + 8 * 64
    );
}

#[test]
fn test_partial_eq() {
    let mut account_data = vec![0u8; ZeroCopyCyclicVecU64::<u64>::required_size_for_capacity(4)];
    let mut vec = ZeroCopyCyclicVecU64::<u64>::new(4, &mut account_data).unwrap();
    for i in 0..5 {
        vec.push(i as u64 % 4);
    }

    let mut account_data = vec![0u8; ZeroCopyCyclicVecU64::<u64>::required_size_for_capacity(4)];
    let mut vec2 = ZeroCopyCyclicVecU64::<u64>::new(4, &mut account_data).unwrap();
    for i in 0..4 {
        vec2.push(i as u64);
    }

    // assert that current index is included in equality check
    // -> values are the same but current index is different
    assert_ne!(vec, vec2);
    assert_eq!(vec.as_slice(), vec2.as_slice());

    vec.clear();
    for i in 0..4 {
        vec.push(i as u64);
    }
    assert_eq!(vec, vec2);
}
