// Edge case: Multiple Option fields with optimized types
#![cfg(feature="mut")] 
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct NestedOptions {
    pub opt_u16: Option<u16>,
    pub opt_u32: Option<u32>,
    pub opt_u64: Option<u64>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = NestedOptions {
        opt_u16: Some(100),
        opt_u32: Some(200),
        opt_u64: None,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = NestedOptions::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Option fields
    assert!(remaining.is_empty());
    
    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = NestedOptions::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}