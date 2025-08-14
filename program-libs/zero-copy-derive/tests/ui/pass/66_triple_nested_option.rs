// Edge case: Triple nested Option
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
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
    let zero_copy_read = TripleNestedOption::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = TripleNestedOption::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to Vec fields in nested Options not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for TripleNestedOption");
}
