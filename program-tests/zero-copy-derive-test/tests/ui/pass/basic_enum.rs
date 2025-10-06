use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq typically not used with enums
#[repr(C)]
pub enum BasicEnum {
    UnitVariant,
    SingleField(u32),
    AnotherField(u64),
}

fn main() {
    // Test Borsh compatibility
    let ref_enum = BasicEnum::SingleField(42);
    let bytes = ref_enum.try_to_vec().unwrap();

    let (_enum_copy, remaining) = BasicEnum::zero_copy_at(&bytes).unwrap();
    assert!(remaining.is_empty());
}
