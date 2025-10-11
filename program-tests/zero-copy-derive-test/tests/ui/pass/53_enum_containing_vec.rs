// Edge case: Enum containing Vec

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum EnumWithVec {
    Empty,
    Data(Vec<u8>),
    MoreData(Vec<u32>),
}

fn main() {
    let instance = EnumWithVec::Data(vec![1, 2, 3, 4, 5]);

    // Test Borsh serialization
    let bytes = instance.try_to_vec().unwrap();

    // Test zero_copy_at
    let (_zero_copy_instance, _remaining) = EnumWithVec::zero_copy_at(&bytes).unwrap();
    assert!(_remaining.is_empty());
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation for enums

    // Note: Enums do not support ZeroCopyEq
    println!("✓ EnumWithVec Borsh compatibility test passed");
}
