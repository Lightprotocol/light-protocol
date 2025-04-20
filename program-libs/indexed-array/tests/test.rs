use std::{cmp::Ordering, str::FromStr};

use light_hasher::{bigint::bigint_to_be_bytes_array, Hasher, Poseidon};
use light_indexed_array::{
    array::{IndexedArray, IndexedElement},
    errors::IndexedArrayError,
    HIGHEST_ADDRESS_PLUS_ONE,
};
use num_bigint::{BigUint, RandBigInt, ToBigUint};
use rand::thread_rng;

#[test]
pub fn hash_reference_indexed_element() {
    let element = IndexedElement::<usize> {
        value: 0.to_biguint().unwrap(),
        index: 0,
        next_index: 1,
    };

    let next_value = BigUint::from_str(HIGHEST_ADDRESS_PLUS_ONE).unwrap();
    let hash = element.hash::<Poseidon>(&next_value).unwrap();
    assert_eq!(
        hash,
        [
            20, 60, 11, 236, 225, 135, 154, 131, 147, 160, 45, 8, 88, 53, 104, 12, 211, 241, 51, 6,
            246, 74, 149, 120, 67, 52, 190, 125, 51, 177, 204, 231
        ]
    );
}

#[test]
fn test_indexed_element_cmp() {
    let mut rng = thread_rng();

    for _ in 0..1000 {
        let value = rng.gen_biguint(128);
        let element_1 = IndexedElement::<u16> {
            index: 0,
            value: value.clone(),
            next_index: 1,
        };
        let element_2 = IndexedElement::<u16> {
            index: 0,
            value,
            next_index: 1,
        };
        assert_eq!(element_1, element_2);
        assert_eq!(element_2, element_1);
        assert!(matches!(element_1.cmp(&element_2), Ordering::Equal));
        assert!(matches!(element_2.cmp(&element_1), Ordering::Equal));

        let value_higher = rng.gen_biguint(128);
        if value_higher == 0.to_biguint().unwrap() {
            continue;
        }
        let value_lower = rng.gen_biguint_below(&value_higher);
        let element_lower = IndexedElement::<u16> {
            index: 0,
            value: value_lower,
            next_index: 1,
        };
        let element_higher = IndexedElement::<u16> {
            index: 1,
            value: value_higher,
            next_index: 2,
        };
        assert_ne!(element_lower, element_higher);
        assert_ne!(element_higher, element_lower);
        assert!(matches!(element_lower.cmp(&element_higher), Ordering::Less));
        assert!(matches!(
            element_higher.cmp(&element_lower),
            Ordering::Greater
        ));
        assert!(matches!(
            element_lower.partial_cmp(&element_higher),
            Some(Ordering::Less)
        ));
        assert!(matches!(
            element_higher.partial_cmp(&element_lower),
            Some(Ordering::Greater)
        ));
    }
}

/// Tests the insertion of elements to the indexing array.
#[test]
fn test_append() {
    // The initial state of the array looks like:
    //
    // ```
    // value      = [0] [0] [0] [0] [0] [0] [0] [0]
    // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
    // ```
    let mut indexed_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

    let nullifier1 = 30_u32.to_biguint().unwrap();
    let bundle1 = indexed_array.new_element(&nullifier1).unwrap();
    assert!(indexed_array.find_element(&nullifier1).is_none());
    indexed_array.append(&nullifier1).unwrap();

    // After adding a new value 30, it should look like:
    //
    // ```
    // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
    // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low element is the first node, with index 0 and value 0. There is
    //   no node with value greater as 30, so we found it as a one pointing to
    //   node 0 (which will always have value 0).
    // * The new nullifier is inserted in index 1.
    // * `next_*` fields of the low nullifier are updated to point to the new
    //   nullifier.
    assert_eq!(
        indexed_array.find_element(&nullifier1),
        Some(&bundle1.new_element),
    );
    let expected_hash = Poseidon::hashv(&[
        bigint_to_be_bytes_array::<32>(&nullifier1)
            .unwrap()
            .as_ref(),
        bigint_to_be_bytes_array::<32>(&(0.to_biguint().unwrap()))
            .unwrap()
            .as_ref(),
    ])
    .unwrap();
    assert_eq!(indexed_array.hash_element(1).unwrap(), expected_hash);
    assert_eq!(
        indexed_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 1,
        },
    );
    assert_eq!(
        indexed_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexed_array.iter().collect::<Vec<_>>().as_slice(),
        &[
            &IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 1,
            },
            &IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0
            }
        ]
    );

    let nullifier2 = 10_u32.to_biguint().unwrap();
    let bundle2 = indexed_array.new_element(&nullifier2).unwrap();
    assert!(indexed_array.find_element(&nullifier2).is_none());
    indexed_array.append(&nullifier2).unwrap();

    // After adding an another value 10, it should look like:
    //
    // ```
    // value      = [ 0] [30] [10] [0] [0] [0] [0] [0]
    // next_index = [ 2] [ 0] [ 1] [0] [0] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low nullifier is still the node 0, but this time for differen reason -
    //   its `next_index` 2 contains value 30, whish is greater than 10.
    // * The new nullifier is inserted as node 2.
    // * Low nullifier is pointing to the index 1. We assign the 1st nullifier
    //   as the next nullifier of our new nullifier. Therefore, our new nullifier
    //   looks like: `[value = 10, next_index = 1]`.
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 0, next_index = 2]`.
    // * The previously inserted nullifier, the node 1, remains unchanged.
    assert_eq!(
        indexed_array.find_element(&nullifier2),
        Some(&bundle2.new_element),
    );
    let expected_hash = Poseidon::hashv(&[
        bigint_to_be_bytes_array::<32>(&nullifier2)
            .unwrap()
            .as_ref(),
        bigint_to_be_bytes_array::<32>(&(30.to_biguint().unwrap()))
            .unwrap()
            .as_ref(),
    ])
    .unwrap();
    assert_eq!(indexed_array.hash_element(2).unwrap(), expected_hash);
    assert_eq!(
        indexed_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexed_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexed_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );
    assert_eq!(
        indexed_array.iter().collect::<Vec<_>>().as_slice(),
        &[
            &IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            },
            &IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            },
            &IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 1,
            }
        ]
    );

    let nullifier3 = 20_u32.to_biguint().unwrap();
    let bundle3 = indexed_array.new_element(&nullifier3).unwrap();
    assert!(indexed_array.find_element(&nullifier3).is_none());
    indexed_array.append(&nullifier3).unwrap();

    // After adding an another value 20, it should look like:
    //
    // ```
    // value      = [ 0] [30] [10] [20] [0] [0] [0] [0]
    // next_index = [ 2] [ 0] [ 3] [ 1] [0] [0] [0] [0]
    // ```
    //
    // Because:
    // * Low nullifier is the node 2.
    // * The new nullifier is inserted as node 3.
    // * Low nullifier is pointing to the node 2. We assign the 1st nullifier
    //   as the next nullifier of our new nullifier. Therefore, our new
    //   nullifier looks like:
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 10, next_index = 3]`.
    assert_eq!(
        indexed_array.find_element(&nullifier3),
        Some(&bundle3.new_element),
    );
    let expected_hash = Poseidon::hashv(&[
        bigint_to_be_bytes_array::<32>(&nullifier3)
            .unwrap()
            .as_ref(),
        bigint_to_be_bytes_array::<32>(&(30.to_biguint().unwrap()))
            .unwrap()
            .as_ref(),
    ])
    .unwrap();
    assert_eq!(indexed_array.hash_element(3).unwrap(), expected_hash);
    assert_eq!(
        indexed_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexed_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexed_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 3,
        }
    );
    assert_eq!(
        indexed_array.elements[3],
        IndexedElement {
            index: 3,
            value: 20_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );
    assert_eq!(
        indexed_array.iter().collect::<Vec<_>>().as_slice(),
        &[
            &IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            },
            &IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 0,
            },
            &IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            },
            &IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1
            }
        ]
    );

    let nullifier4 = 50_u32.to_biguint().unwrap();
    let bundle4 = indexed_array.new_element(&nullifier4).unwrap();
    assert!(indexed_array.find_element(&nullifier4).is_none());
    indexed_array.append(&nullifier4).unwrap();

    // After adding an another value 50, it should look like:
    //
    // ```
    // value      = [ 0]  [30] [10] [20] [50] [0] [0] [0]
    // next_index = [ 2]  [ 4] [ 3] [ 1] [0 ] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low nullifier is the node 1 - there is no node with value greater
    //   than 50, so we found it as a one having 0 as the `next_value`.
    // * The new nullifier is inserted as node 4.
    // * Low nullifier is not pointing to any node. So our new nullifier
    //   is not going to point to any other node either. Therefore, the new
    //   nullifier looks like: `[value = 50, next_index = 0]`.
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 30, next_index = 4]`.
    assert_eq!(
        indexed_array.find_element(&nullifier4),
        Some(&bundle4.new_element),
    );
    let expected_hash = Poseidon::hashv(&[
        bigint_to_be_bytes_array::<32>(&nullifier4)
            .unwrap()
            .as_ref(),
        bigint_to_be_bytes_array::<32>(&(0.to_biguint().unwrap()))
            .unwrap()
            .as_ref(),
    ])
    .unwrap();
    assert_eq!(indexed_array.hash_element(4).unwrap(), expected_hash);
    assert_eq!(
        indexed_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexed_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 4,
        }
    );
    assert_eq!(
        indexed_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 3,
        }
    );
    assert_eq!(
        indexed_array.elements[3],
        IndexedElement {
            index: 3,
            value: 20_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );
    assert_eq!(
        indexed_array.elements[4],
        IndexedElement {
            index: 4,
            value: 50_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexed_array.iter().collect::<Vec<_>>().as_slice(),
        &[
            &IndexedElement {
                index: 0,
                value: 0_u32.to_biguint().unwrap(),
                next_index: 2,
            },
            &IndexedElement {
                index: 1,
                value: 30_u32.to_biguint().unwrap(),
                next_index: 4,
            },
            &IndexedElement {
                index: 2,
                value: 10_u32.to_biguint().unwrap(),
                next_index: 3,
            },
            &IndexedElement {
                index: 3,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            },
            &IndexedElement {
                index: 4,
                value: 50_u32.to_biguint().unwrap(),
                next_index: 0,
            }
        ]
    );
}

#[test]
fn test_append_with_low_element_index() {
    // The initial state of the array looks like:
    //
    // ```
    // value      = [0] [0] [0] [0] [0] [0] [0] [0]
    // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
    // ```
    let mut indexing_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

    let low_element_index = 0;
    let nullifier1 = 30_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier1)
        .unwrap();

    // After adding a new value 30, it should look like:
    //
    // ```
    // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
    // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low element is the first node, with index 0 and value 0. There is
    //   no node with value greater as 30, so we found it as a one pointing to
    //   node 0 (which will always have value 0).
    // * The new nullifier is inserted in index 1.
    // * `next_*` fields of the low nullifier are updated to point to the new
    //   nullifier.
    assert_eq!(
        indexing_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 1,
        },
    );
    assert_eq!(
        indexing_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );

    let low_element_index = 0;
    let nullifier2 = 10_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier2)
        .unwrap();

    // After adding an another value 10, it should look like:
    //
    // ```
    // value      = [ 0] [30] [10] [0] [0] [0] [0] [0]
    // next_index = [ 2] [ 0] [ 1] [0] [0] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low nullifier is still the node 0, but this time for differen reason -
    //   its `next_index` 2 contains value 30, whish is greater than 10.
    // * The new nullifier is inserted as node 2.
    // * Low nullifier is pointing to the index 1. We assign the 1st nullifier
    //   as the next nullifier of our new nullifier. Therefore, our new nullifier
    //   looks like: `[value = 10, next_index = 1]`.
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 0, next_index = 2]`.
    // * The previously inserted nullifier, the node 1, remains unchanged.
    assert_eq!(
        indexing_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexing_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexing_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );

    let low_element_index = 2;
    let nullifier3 = 20_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier3)
        .unwrap();

    // After adding an another value 20, it should look like:
    //
    // ```
    // value      = [ 0] [30] [10] [20] [0] [0] [0] [0]
    // next_index = [ 2] [ 0] [ 3] [ 1] [0] [0] [0] [0]
    // ```
    //
    // Because:
    // * Low nullifier is the node 2.
    // * The new nullifier is inserted as node 3.
    // * Low nullifier is pointing to the node 2. We assign the 1st nullifier
    //   as the next nullifier of our new nullifier. Therefore, our new
    //   nullifier looks like:
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 10, next_index = 3]`.
    assert_eq!(
        indexing_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexing_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
    assert_eq!(
        indexing_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 3,
        }
    );
    assert_eq!(
        indexing_array.elements[3],
        IndexedElement {
            index: 3,
            value: 20_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );

    let low_element_index = 1;
    let nullifier4 = 50_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier4)
        .unwrap();

    // After adding an another value 50, it should look like:
    //
    // ```
    // value      = [ 0]  [30] [10] [20] [50] [0] [0] [0]
    // next_index = [ 2]  [ 4] [ 3] [ 1] [0 ] [0] [0] [0]
    // ```
    //
    // Because:
    //
    // * Low nullifier is the node 1 - there is no node with value greater
    //   than 50, so we found it as a one having 0 as the `next_value`.
    // * The new nullifier is inserted as node 4.
    // * Low nullifier is not pointing to any node. So our new nullifier
    //   is not going to point to any other node either. Therefore, the new
    //   nullifier looks like: `[value = 50, next_index = 0]`.
    // * Low nullifier is updated to point to the new nullifier. Therefore,
    //   after update it looks like: `[value = 30, next_index = 4]`.
    assert_eq!(
        indexing_array.elements[0],
        IndexedElement {
            index: 0,
            value: 0_u32.to_biguint().unwrap(),
            next_index: 2,
        }
    );
    assert_eq!(
        indexing_array.elements[1],
        IndexedElement {
            index: 1,
            value: 30_u32.to_biguint().unwrap(),
            next_index: 4,
        }
    );
    assert_eq!(
        indexing_array.elements[2],
        IndexedElement {
            index: 2,
            value: 10_u32.to_biguint().unwrap(),
            next_index: 3,
        }
    );
    assert_eq!(
        indexing_array.elements[3],
        IndexedElement {
            index: 3,
            value: 20_u32.to_biguint().unwrap(),
            next_index: 1,
        }
    );
    assert_eq!(
        indexing_array.elements[4],
        IndexedElement {
            index: 4,
            value: 50_u32.to_biguint().unwrap(),
            next_index: 0,
        }
    );
}

/// Tries to violate the integrity of the array by pointing to invalid low
/// nullifiers. Tests whether the range check works correctly and disallows
/// the invalid appends from happening.
#[test]
fn test_append_with_low_element_index_invalid() {
    // The initial state of the array looks like:
    //
    // ```
    // value      = [0] [0] [0] [0] [0] [0] [0] [0]
    // next_index = [0] [0] [0] [0] [0] [0] [0] [0]
    // ```
    let mut indexing_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

    // Append nullifier 30. The low nullifier is at index 0. The array
    // should look like:
    //
    // ```
    // value      = [ 0] [30] [0] [0] [0] [0] [0] [0]
    // next_index = [ 1] [ 0] [0] [0] [0] [0] [0] [0]
    // ```
    let low_element_index = 0;
    let nullifier1 = 30_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier1)
        .unwrap();

    // Try appending nullifier 20, while pointing to index 1 as low
    // nullifier.
    // Therefore, the new element is lower than the supposed low element.
    let low_element_index = 1;
    let nullifier2 = 20_u32.to_biguint().unwrap();
    assert!(matches!(
        indexing_array.append_with_low_element_index(low_element_index, &nullifier2),
        Err(IndexedArrayError::LowElementGreaterOrEqualToNewElement)
    ));

    // Try appending nullifier 50, while pointing to index 0 as low
    // nullifier.
    // Therefore, the new element is greater than next element.
    let low_element_index = 0;
    let nullifier2 = 50_u32.to_biguint().unwrap();
    assert!(matches!(
        indexing_array.append_with_low_element_index(low_element_index, &nullifier2),
        Err(IndexedArrayError::NewElementGreaterOrEqualToNextElement),
    ));

    // Append nullifier 50 correctly, with 0 as low nullifier. The array
    // should look like:
    //
    // ```
    // value      = [ 0] [30] [50] [0] [0] [0] [0] [0]
    // next_index = [ 1] [ 2] [ 0] [0] [0] [0] [0] [0]
    // ```
    let low_element_index = 1;
    let nullifier2 = 50_u32.to_biguint().unwrap();
    indexing_array
        .append_with_low_element_index(low_element_index, &nullifier2)
        .unwrap();

    // Try appending nullifier 40, while pointint to index 2 (value 50) as
    // low nullifier.
    // Therefore, the pointed low element is greater than the new element.
    let low_element_index = 2;
    let nullifier3 = 40_u32.to_biguint().unwrap();
    assert!(matches!(
        indexing_array.append_with_low_element_index(low_element_index, &nullifier3),
        Err(IndexedArrayError::LowElementGreaterOrEqualToNewElement)
    ));
}

/// Tests whether `find_*_for_existent` elements return `None` when a
/// nonexistent is provided.
#[test]
fn test_find_low_element_for_existent_element() {
    let mut indexed_array: IndexedArray<Poseidon, usize> = IndexedArray::default();

    // Append nullifiers 40 and 20.
    let low_element_index = 0;
    let nullifier_1 = 40_u32.to_biguint().unwrap();
    indexed_array
        .append_with_low_element_index(low_element_index, &nullifier_1)
        .unwrap();
    let low_element_index = 0;
    let nullifier_2 = 20_u32.to_biguint().unwrap();
    indexed_array
        .append_with_low_element_index(low_element_index, &nullifier_2)
        .unwrap();

    // Try finding a low element for nonexistent nullifier 30.
    let nonexistent_nullifier = 30_u32.to_biguint().unwrap();
    // `*_existent` methods should fail.
    let res = indexed_array.find_low_element_index_for_existent(&nonexistent_nullifier);
    assert!(matches!(res, Err(IndexedArrayError::ElementDoesNotExist)));
    let res = indexed_array.find_low_element_for_existent(&nonexistent_nullifier);
    assert!(matches!(res, Err(IndexedArrayError::ElementDoesNotExist)));
    // `*_nonexistent` methods should succeed.
    let low_element_index = indexed_array
        .find_low_element_index_for_nonexistent(&nonexistent_nullifier)
        .unwrap();
    assert_eq!(low_element_index, 2);
    let low_element = indexed_array
        .find_low_element_for_nonexistent(&nonexistent_nullifier)
        .unwrap();
    assert_eq!(
        low_element,
        (
            IndexedElement::<usize> {
                index: 2,
                value: 20_u32.to_biguint().unwrap(),
                next_index: 1,
            },
            40_u32.to_biguint().unwrap(),
        )
    );

    // Try finding a low element of existent nullifier 40.
    // `_existent` methods should succeed.
    let low_element_index = indexed_array
        .find_low_element_index_for_existent(&nullifier_1)
        .unwrap();
    assert_eq!(low_element_index, 2);
    let low_element = indexed_array
        .find_low_element_for_existent(&nullifier_1)
        .unwrap();
    assert_eq!(
        low_element,
        IndexedElement::<usize> {
            index: 2,
            value: 20_u32.to_biguint().unwrap(),
            next_index: 1,
        },
    );
    // `*_nonexistent` methods should fail.
    let res = indexed_array.find_low_element_index_for_nonexistent(&nullifier_1);
    assert!(matches!(res, Err(IndexedArrayError::ElementAlreadyExists)));
    let res = indexed_array.find_low_element_for_nonexistent(&nullifier_1);
    assert!(matches!(res, Err(IndexedArrayError::ElementAlreadyExists)));
}
