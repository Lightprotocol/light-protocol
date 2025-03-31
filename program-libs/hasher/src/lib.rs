pub mod bigint;
mod data_hasher;
pub mod errors;
pub mod hash_to_field_size;
pub mod keccak;
pub mod poseidon;
pub mod sha256;
pub mod syscalls;
pub mod to_byte_array;
pub mod zero_bytes;
pub mod zero_indexed_leaf;

pub use data_hasher::DataHasher;
pub use keccak::Keccak;
pub use poseidon::Poseidon;
pub use sha256::Sha256;

pub use crate::errors::HasherError;
use crate::zero_bytes::ZeroBytes;

pub const HASH_BYTES: usize = 32;

pub type Hash = [u8; HASH_BYTES];

pub trait Hasher {
    fn hash(val: &[u8]) -> Result<Hash, HasherError>;
    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError>;
    fn zero_bytes() -> ZeroBytes;
    fn zero_indexed_leaf() -> [u8; 32];
}

// TODO: remove once light-sdk is switched to account-checks
pub trait Discriminator {
    const DISCRIMINATOR: [u8; 8];
    fn discriminator() -> [u8; 8] {
        Self::DISCRIMINATOR
    }
}

#[cfg(all(
    feature = "pinocchio",
    not(feature = "solana"),
    not(feature = "anchor")
))]
use pinocchio::program_error::ProgramError;

#[cfg(all(
    feature = "anchor",
    not(feature = "solana"),
    not(feature = "pinocchio")
))]
use anchor_lang::prelude::{ProgramError, Pubkey};
#[cfg(all(
    feature = "solana",
    not(feature = "anchor"),
    not(feature = "pinocchio")
))]
use solana_program::{program_error::ProgramError, pubkey::Pubkey};

const _: () = {
    #[cfg(any(
        all(feature = "solana", feature = "anchor"),
        all(feature = "solana", feature = "pinocchio"),
        all(feature = "anchor", feature = "pinocchio")
    ))]
    {
        #[cfg(all(feature = "solana", feature = "anchor"))]
        compile_error!("Only one feature among 'solana', 'anchor'' may be active.");
        #[cfg(all(feature = "solana", feature = "pinocchio"))]
        compile_error!("Only one feature among 'solana', 'and 'pinocchio' may be active.");
        #[cfg(all(feature = "anchor", feature = "pinocchio"))]
        compile_error!("Only one feature among 'anchor', and 'pinocchio' may be active.");
    }
    #[cfg(not(any(feature = "solana", feature = "anchor", feature = "pinocchio")))]
    compile_error!("Exactly one of 'solana', 'anchor', or 'pinocchio' must be enabled.");
};
