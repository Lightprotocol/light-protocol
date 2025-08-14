#![cfg(feature = "mut")]
use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Create a simple Pubkey type for testing with all required traits
use light_zero_copy::{KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes};

#[derive(Debug, PartialEq, Clone, Copy, KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes, BorshSerialize, BorshDeserialize)]
#[repr(C)]
pub struct Pubkey([u8; 32]);

#[derive(Debug, ZeroCopy, ZeroCopyMut, BorshSerialize, BorshDeserialize)]
// Note: ZeroCopyEq not supported for Vec<primitive> fields
#[repr(C)]
pub struct WithPubkey {
    pub owner: Pubkey,
    pub amount: u64,
    pub flags: Vec<bool>,
}

fn main() {
    // Test Borsh compatibility
    let ref_struct = WithPubkey {
        owner: Pubkey([1; 32]),
        amount: 1000,
        flags: vec![true, false, true],
    };
    let bytes = ref_struct.try_to_vec().unwrap();

    let (_struct_copy, remaining) = WithPubkey::zero_copy_at(&bytes).unwrap();
    // Note: Can't use assert_eq! due to ZeroCopyEq limitation with Vec<primitive>
    assert!(remaining.is_empty());

    let mut bytes_mut = bytes.clone();
    let (_struct_copy_mut, remaining) = WithPubkey::zero_copy_at_mut(&mut bytes_mut).unwrap();
    assert!(remaining.is_empty());
}