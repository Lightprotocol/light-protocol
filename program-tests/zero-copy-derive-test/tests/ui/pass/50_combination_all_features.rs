// Edge case: Combination of all features

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
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

    // assert byte len
    let config = CombinationAllFeaturesConfig {
        vec1: 3,
        opt1: true,
        vec2: 2,
        opt2: true,
        vec3: 2,
        opt3: (true, vec![(); 2]),
    };
    let byte_len = CombinationAllFeatures::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        CombinationAllFeatures::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.meta1 = 42.into();
    struct_copy_mut.meta2 = 1; // true as u8
    struct_copy_mut.meta3[0] = 1;
    struct_copy_mut.meta3[1] = 2;
    struct_copy_mut.meta3[2] = 3;
    struct_copy_mut.meta3[3] = 4;
    struct_copy_mut.vec1[0] = 10;
    struct_copy_mut.vec1[1] = 20;
    struct_copy_mut.vec1[2] = 30;
    if let Some(ref mut val) = struct_copy_mut.opt1 {
        **val = 100.into();
    }
    struct_copy_mut.vec2[0] = 200.into();
    struct_copy_mut.vec2[1] = 300.into();
    if let Some(ref mut val) = struct_copy_mut.opt2 {
        **val = 400.into();
    }
    *struct_copy_mut.pubkey = Pubkey([1; 32]);
    struct_copy_mut.vec3[0] = 1; // true as u8
    struct_copy_mut.vec3[1] = 0; // false as u8
    if let Some(ref mut data) = struct_copy_mut.opt3 {
        *data[0] = 50;
        *data[1] = 60;
    }
    for i in 0..16 {
        struct_copy_mut.arr1[i] = 999.into();
    }
    *struct_copy_mut.final1 = 777.into();
    *struct_copy_mut.final2 = 0; // false as u8
    assert_eq!(new_bytes, bytes);

    // Note: Cannot use assert_eq! with entire structs due to array, Vec, and Pubkey fields
    println!("✓ CombinationAllFeatures Borsh compatibility test passed");
}
