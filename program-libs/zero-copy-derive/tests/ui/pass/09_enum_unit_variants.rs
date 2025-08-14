// Edge case: Enum with only unit variants
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum UnitEnum {
    First,
    Second,
    Third,
    Fourth,
    Fifth,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = UnitEnum::Third;
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = UnitEnum::zero_copy_at(&bytes).unwrap();
    // Note: ZeroCopyEq not supported for enums
    assert!(remaining.is_empty());

    // Note: ZeroCopyMut not supported for enums
}
