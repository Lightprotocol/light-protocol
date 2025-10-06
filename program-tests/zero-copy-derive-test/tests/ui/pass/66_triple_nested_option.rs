// Edge case: Triple nested Option

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct TripleNestedOption {
    pub deeply_nested: Option<Option<Option<u32>>>,
    pub also_nested: Option<Option<Vec<u8>>>,
}

fn main() {
    let original = TripleNestedOption {
        deeply_nested: Some(Some(Some(42))),
        also_nested: Some(Some(vec![1, 2, 3, 4])),
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = TripleNestedOption::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = TripleNestedOption::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // assert byte len
    let config = TripleNestedOptionConfig {
        deeply_nested: (true, (true, (true, ()))),
        also_nested: (true, (true, vec![(); 4])),
    };
    let byte_len = TripleNestedOption::byte_len(&config).unwrap();
    assert_eq!(serialized.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        TripleNestedOption::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut second_level) = struct_copy_mut.deeply_nested {
        if let Some(ref mut third_level) = second_level {
            if let Some(ref mut val) = third_level {
                **val = 42.into();
            }
        }
    }
    if let Some(ref mut second_level) = struct_copy_mut.also_nested {
        if let Some(ref mut data) = *second_level {
            *data[0] = 1;
            *data[1] = 2;
            *data[2] = 3;
            *data[3] = 4;
        }
    }
    assert_eq!(new_bytes, serialized);
}
