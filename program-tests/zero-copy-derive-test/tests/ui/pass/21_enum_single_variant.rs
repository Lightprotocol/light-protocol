// Edge case: Enum with single variant

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq typically not used with enums
#[repr(C)]
pub enum SingleVariant {
    Only,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = SingleVariant::Only;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = SingleVariant::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());
}
