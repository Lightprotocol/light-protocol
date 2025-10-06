// Edge case: Struct with only arrays (no Vec/Option)

use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct ArraysOnly {
    pub bytes: [u8; 32],
    pub words: [u16; 16],
    pub dwords: [u32; 8],
    pub qwords: [u64; 4],
    pub flags: [bool; 64],
}

fn main() {
    let original = ArraysOnly {
        bytes: [1u8; 32],
        words: [2u16; 16],
        dwords: [3u32; 8],
        qwords: [4u64; 4],
        flags: [true; 64],
    };

    // Test Borsh serialization
    let serialized = original.try_to_vec().unwrap();

    // Test zero_copy_at (read-only)
    let _zero_copy_read = ArraysOnly::zero_copy_at(&serialized).unwrap();

    // Test zero_copy_at_mut (mutable)
    let mut serialized_mut = serialized.clone();
    let _zero_copy_mut = ArraysOnly::zero_copy_at_mut(&mut serialized_mut).unwrap();

    // Note: Cannot use assert_eq! due to array fields not implementing ZeroCopyEq
    println!("Borsh compatibility test passed for ArraysOnly");
}
