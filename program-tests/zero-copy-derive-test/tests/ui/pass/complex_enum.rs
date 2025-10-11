use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq typically not used with enums
#[repr(C)]
pub enum ComplexEnum {
    UnitVariant,
    U64Field(u64),
    BoolField(bool),
    U32Field(u32),
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = ComplexEnum::U64Field(12345);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = ComplexEnum::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());
}
