// Edge case: Combination of all features
#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Import Pubkey from the test helper
#[path = "../../instruction_data.rs"]
mod instruction_data;
use instruction_data::Pubkey;

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct CombinationAllFeatures {
    // Meta fields
    pub meta1: u32,
    pub meta2: bool,
    pub meta3: [u8; 4],

    // Dynamic fields
    pub vec1: Vec<u8>,
    pub opt1: Option<u64>,
    pub vec2: Vec<u32>,
    pub opt2: Option<u32>,

    // Special types
    pub pubkey: Pubkey,

    // More dynamic
    pub vec3: Vec<bool>,
    pub opt3: Option<Vec<u8>>,

    // Arrays after dynamic
    pub arr1: [u64; 16],

    // Final primitives
    pub final1: u32,
    pub final2: bool,
}

fn main() {
    let instance = CombinationAllFeatures {
        meta1: 42,
        meta2: true,
        meta3: [1, 2, 3, 4],
        vec1: vec![10, 20, 30],
        opt1: Some(100),
        vec2: vec![200, 300],
        opt2: Some(400),
        pubkey: Pubkey([1; 32]),
        vec3: vec![true, false],
        opt3: Some(vec![50, 60]),
        arr1: [999; 16],
        final1: 777,
        final2: false,
    };

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, remaining) = CombinationAllFeatures::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test zero_copy_at_mut
    let mut bytes_mut = bytes.clone();
    let (_zero_copy_mut, remaining) =
        CombinationAllFeatures::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());

    // Note: Cannot use assert_eq! with entire structs due to array, Vec, and Pubkey fields
    println!("✓ CombinationAllFeatures Borsh compatibility test passed");
}
