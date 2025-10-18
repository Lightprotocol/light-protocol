#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(feature = "alloc", not(feature = "std")))]
extern crate alloc;

#[cfg(feature = "alloc")]
#[cfg(not(feature = "std"))]
pub use alloc::{string::String, vec, vec::Vec};
#[cfg(feature = "std")]
pub use std::{string::String, vec, vec::Vec};

pub mod bigint;
mod data_hasher;
pub mod errors;
pub mod hash_chain;
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
    const ID: u8;
    fn hash(val: &[u8]) -> Result<Hash, HasherError>;
    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError>;
    fn zero_bytes() -> ZeroBytes;
    fn zero_indexed_leaf() -> [u8; 32];
}
