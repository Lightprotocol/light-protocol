// Edge case: Deep nesting of Options

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct DeepNesting {
    pub nested: Option<Option<u32>>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = DeepNesting {
        nested: Some(Some(42)),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = DeepNesting::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = DeepNesting::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = DeepNestingConfig {
        nested: (true, (true, ())),
    };
    let byte_len = DeepNesting::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        DeepNesting::new_zero_copy(&mut new_bytes, config).unwrap();
    // set nested Option<Option<u32>> field
    if let Some(ref mut outer) = struct_copy_mut.nested {
        if let Some(ref mut inner) = outer {
            **inner = 42.into();
        }
    }
    assert_eq!(new_bytes, bytes);
}
