// Edge case: Option containing arrays
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct OptionOfArray {
    pub maybe_bytes: Option<[u8; 32]>,
    pub maybe_nums: Option<[u32; 8]>,
    pub maybe_large: Option<[u64; 64]>,
}

fn main() {
    let instance = OptionOfArray {
        maybe_bytes: Some([42; 32]),
        maybe_nums: Some([1, 2, 3, 4, 5, 6, 7, 8]),
        maybe_large: None,
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let deserialized = OptionOfArray::try_from_slice(&bytes).unwrap();
    
    // Test zero_copy_at
    let (zero_copy_instance, remaining) = OptionOfArray::zero_copy_at(&bytes).unwrap();
    assert_eq!(zero_copy_instance.maybe_bytes.is_some(), deserialized.maybe_bytes.is_some());
    assert_eq!(zero_copy_instance.maybe_nums.is_some(), deserialized.maybe_nums.is_some());
    assert_eq!(zero_copy_instance.maybe_large.is_none(), deserialized.maybe_large.is_none());
    assert!(remaining.is_empty());
    
    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (mut zero_copy_mut, remaining) = OptionOfArray::zero_copy_at_mut(&mut bytes_mut).unwrap();
    if let Some(ref mut bytes) = &mut zero_copy_mut.maybe_bytes {
        bytes[0] = 99;
        assert_eq!(bytes[0], 99);
    }
    assert!(remaining.is_empty());
    
    // Note: Cannot use assert_eq! with entire structs due to array fields
    println!("✓ OptionOfArray Borsh compatibility test passed");
}
