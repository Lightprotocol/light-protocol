// Edge case: Single Option field

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct SingleOption {
    pub maybe: Option<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = SingleOption { maybe: Some(12345) };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = SingleOption::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option fields
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = SingleOption::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = SingleOptionConfig { maybe: true };
    let byte_len = SingleOption::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        SingleOption::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    if let Some(ref mut val) = struct_copy_mut.maybe {
        **val = 12345.into();
    }
    assert_eq!(new_bytes, bytes);
}
