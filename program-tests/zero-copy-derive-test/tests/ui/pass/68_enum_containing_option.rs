// Edge case: Enum containing Option

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::ZeroCopyAt;
use light_zero_copy_derive::ZeroCopy;

#[derive(Debug, ZeroCopy, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub enum EnumWithOption {
    Empty,
    MaybeData(Option<u32>),
    MaybeVec(Option<Vec<u8>>),
}

fn main() {
    let original = EnumWithOption::MaybeData(Some(42));

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let (_zero_copy_read, _remaining) = EnumWithOption::zero_copy_at(&serialized).unwrap();

    // Note: Cannot use assert_eq! as enums don't implement ZeroCopyEq
    println!("Borsh compatibility test passed for EnumWithOption");
}
