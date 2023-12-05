pub mod blake3;
pub mod keccak;
pub mod poseidon;
pub mod sha256;

pub use blake3::Blake3;
pub use keccak::Keccak;
pub use poseidon::Poseidon;
pub use sha256::Sha256;

use crate::errors::MerkleTreeError;

pub const HASH_BYTES: usize = 32;

pub type Hash = [u8; HASH_BYTES];

pub trait Hasher {
    fn hash(val: &[u8]) -> Result<Hash, MerkleTreeError>;
    fn hashv(vals: &[&[u8]]) -> Result<Hash, MerkleTreeError>;
}
