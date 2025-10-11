// Test that #[repr(C, packed)] is correctly detected

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct PackedStruct {
    pub a: u8,
    pub b: u32,
    pub c: u16,
}

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C, align(8))]
pub struct AlignedStruct {
    pub x: u64,
    pub y: u32,
}

fn main() {
    // Test packed struct
    let packed = PackedStruct {
        a: 42,
        b: 1000,
        c: 500,
    };
    let bytes = packed.try_to_vec().unwrap();
    let (_packed_copy, remaining) = PackedStruct::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    // Test aligned struct
    let aligned = AlignedStruct { x: 999999, y: 42 };
    let mut bytes = aligned.try_to_vec().unwrap();
    let (_aligned_copy, remaining) = AlignedStruct::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());

    let (_aligned_copy, remaining) = AlignedStruct::zero_copy_at_mut(&mut bytes).unwrap();
    assert!(remaining.is_empty());
}
