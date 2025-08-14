// Edge case: Option<bool>
#![cfg(feature = "mut")]
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

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
}
