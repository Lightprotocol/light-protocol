// Edge case: Very large struct (25+ fields)
#![cfg(feature="mut")] 
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<u8> fields
#[repr(C)]
pub struct LargeStruct {
    pub field_001: u32,
    pub field_002: u32,
    pub field_003: u32,
    pub field_004: u32,
    pub field_005: u32,
    pub field_006: u32,
    pub field_007: u32,
    pub field_008: u32,
    pub field_009: u32,
    pub field_010: u32,
    pub field_011: u32,
    pub field_012: u32,
    pub field_013: u32,
    pub field_014: u32,
    pub field_015: u32,
    pub field_016: u32,
    pub field_017: u32,
    pub field_018: u32,
    pub field_019: u32,
    pub field_020: u32,
    pub field_021: Vec<u8>,
    pub field_022: Option<u64>,
    pub field_023: u32,
    pub field_024: u32,
    pub field_025: u32,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = LargeStruct {
        field_001: 1, field_002: 2, field_003: 3, field_004: 4, field_005: 5,
        field_006: 6, field_007: 7, field_008: 8, field_009: 9, field_010: 10,
        field_011: 11, field_012: 12, field_013: 13, field_014: 14, field_015: 15,
        field_016: 16, field_017: 17, field_018: 18, field_019: 19, field_020: 20,
        field_021: vec![1, 2, 3],
        field_022: Some(22),
        field_023: 23, field_024: 24, field_025: 25,
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = LargeStruct::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec fields
    assert!(remaining.is_empty());
    
    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = LargeStruct::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}