// Edge case: Option<bool>

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq has limitations with Option<bool> fields
#[repr(C)]
pub struct OptionBool {
    pub maybe_flag: Option<bool>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = OptionBool {
        maybe_flag: Some(true),
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = OptionBool::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option<bool>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = OptionBool::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // assert byte len
    let config = OptionBoolConfig {
        maybe_flag: (true, ()),
    };
    let byte_len = OptionBool::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        OptionBool::new_zero_copy(&mut new_bytes, config).unwrap();
    // set Option<bool> field value
    if let Some(ref mut val) = struct_copy_mut.maybe_flag {
        **val = 1; // true as u8
    }
    assert_eq!(new_bytes, bytes);
}
