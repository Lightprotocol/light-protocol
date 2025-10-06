// Edge case: Many enum variants (testing discriminant handling)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for enums
#[repr(C)]
pub enum ManyVariants {
    V0,
    V1,
    V2,
    V3,
    V4,
    V5,
    V6,
    V7,
    V8,
    V9,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = ManyVariants::V5;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = ManyVariants::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums
    assert!(remaining.is_empty());
}
