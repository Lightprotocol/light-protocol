// Edge case: Enum with mixed variant types
use light_zero_copy_derive::ZeroCopy;
use borsh::{BorshSerialize, BorshDeserialize};
use light_zero_copy::traits::ZeroCopyAt;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum MixedEnum {
    Empty,
    WithData(u32),
    Another,
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = MixedEnum::WithData(42);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = MixedEnum::zero_copy_at(&bytes).unwrap();
    // Note: ZeroCopyEq not supported for enums
    assert!(remaining.is_empty());
    
    // Note: ZeroCopyMut not supported for enums
}