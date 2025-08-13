use light_zero_copy_derive::ZeroCopy;

// Create a simple Pubkey type for testing with all required traits
use light_zero_copy::{KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes};

#[derive(Debug, PartialEq, Clone, Copy, KnownLayout, Immutable, Unaligned, FromBytes, IntoBytes)]
#[repr(C)]
pub struct Pubkey([u8; 32]);

#[derive(ZeroCopy)]
#[repr(C)]
pub struct WithPubkey {
    pub owner: Pubkey,
    pub amount: u64,
    pub flags: Vec<bool>,
}

fn main() {}