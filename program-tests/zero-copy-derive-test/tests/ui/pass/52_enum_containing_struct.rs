// Edge case: Enum containing struct type

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct InnerData {
    pub field1: u32,
    pub field2: u64,
}

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum EnumWithStruct {
    Empty,
    WithStruct(InnerData),
    Another,
}

fn main() {
    let instance = EnumWithStruct::WithStruct(InnerData {
        field1: 42,
        field2: 12345,
    });

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();
    let _deserialized = EnumWithStruct::try_from_slice(&bytes).unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, _remaining) = EnumWithStruct::zero_copy_at(&bytes).unwrap();
    assert!(_remaining.is_empty());
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums

    // Note: Enums do not support ZeroCopyEq
    println!("✓ EnumWithStruct Borsh compatibility test passed");
}
