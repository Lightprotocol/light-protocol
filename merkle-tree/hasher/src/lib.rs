pub mod errors;
pub mod poseidon;
pub mod sha256;
pub mod zero_bytes;

pub use poseidon::Poseidon;
pub use sha256::Sha256;

use crate::{errors::HasherError, zero_bytes::ZeroBytes};

pub const HASH_BYTES: usize = 32;

pub type Hash = [u8; HASH_BYTES];

pub trait Hasher {
    fn hash(val: &[u8]) -> Result<Hash, HasherError>;
    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError>;
    fn zero_bytes() -> ZeroBytes;
}
