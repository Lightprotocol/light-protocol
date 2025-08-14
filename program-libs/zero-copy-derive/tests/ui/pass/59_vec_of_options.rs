// Edge case: Vec containing Options
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct VecOfOptions {
    pub maybe_values: Vec<Option<u32>>,
    pub maybe_bytes: Vec<Option<u8>>,
    pub maybe_large: Vec<Option<u64>>,
}

fn main() {
    let instance = VecOfOptions {
        maybe_values: vec![Some(1), None, Some(3)],
        maybe_bytes: vec![Some(42), Some(99), None],
        maybe_large: vec![None, Some(12345), Some(67890)],
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = VecOfOptions::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) = VecOfOptions::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to Vec fields
    println!("✓ VecOfOptions Borsh compatibility test passed");
}
