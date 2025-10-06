// Edge case: Enum variant with array

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for enums
#[repr(C)]
pub enum EnumWithArray {
    Empty,
    WithArray([u8; 32]),
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = EnumWithArray::WithArray([42; 32]);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = EnumWithArray::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums
    assert!(remaining.is_empty());
}
