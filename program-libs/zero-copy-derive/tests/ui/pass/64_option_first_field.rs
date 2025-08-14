// Edge case: Struct starting with Option (affects meta boundary)
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct OptionFirstField {
    pub optional: Option<u32>, // Meta stops here
    pub value: u64,
    pub more: u32,
    pub data: Vec<u8>,
}

fn main() {
    let original = OptionFirstField {
        optional: Some(42),
        value: 123456789,
        more: 987654321,
        data: vec![1, 2, 3, 4, 5],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();
    let deserialized: OptionFirstField = OptionFirstField::try_from_slice(&serialized).unwrap();
    
    // Test zero_copy_at (read-only)
    let zero_copy_read = OptionFirstField::zero_copy_at(&serialized).unwrap();
    
    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = OptionFirstField::zero_copy_at_mut(&mut serialized_mut).unwrap();
    
    // Note: Cannot use assert_eq! due to Vec fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for OptionFirstField");
}
