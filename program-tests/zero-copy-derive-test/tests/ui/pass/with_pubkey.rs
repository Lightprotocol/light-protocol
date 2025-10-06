use borsh::{BorshDeserialize, BorshSerialize};
use light_zero_copy::traits::{ZeroCopyAt, ZeroCopyAtMut, ZeroCopyNew};
use light_zero_copy_derive::{ZeroCopy, ZeroCopyMut};

// Create a simple Pubkey type for testing with all required traits
use light_zero_copy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

#[derive(
    Debug,
    PartialEq,
    Clone,
    Copy,
    KnownLayout,
    Immutable,
    Unaligned,
    FromBytes,
    IntoBytes,
    BorshSerialize,
    BorshDeserialize,
)]
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

    // assert byte len
    let config = WithPubkeyConfig { flags: 3 };
    let byte_len = WithPubkey::byte_len(&config).unwrap();
    assert_eq!(bytes.len(), byte_len);
    let mut new_bytes = vec![0u8; byte_len];
    let (mut struct_copy_mut, _remaining) =
        WithPubkey::new_zero_copy(&mut new_bytes, config).unwrap();
    // set field values
    struct_copy_mut.owner = Pubkey([1; 32]);
    struct_copy_mut.amount = 1000.into();
    struct_copy_mut.flags[0] = 1; // true as u8
    struct_copy_mut.flags[1] = 0; // false as u8
    struct_copy_mut.flags[2] = 1; // true as u8
    assert_eq!(new_bytes, bytes);
}
