pub mod bytes;
pub mod errors;
pub mod keccak;
pub mod poseidon;
pub mod sha256;
pub mod syscalls;
pub mod zero_bytes;
pub mod zero_indexed_leaf;

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

pub trait DataHasher {
    fn hash<H: crate::Hasher>(&self) -> Result<[u8; 32], HasherError>;
}

pub trait Discriminator {
    const DISCRIMINATOR: [u8; 8];
    fn discriminator() -> [u8; 8] {
        Self::DISCRIMINATOR
    }
}
