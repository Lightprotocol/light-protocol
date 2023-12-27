use anchor_lang::solana_program::keccak::{hash, hashv};

use crate::{
    errors::HasherError,
    zero_bytes::{keccak::ZERO_BYTES, ZeroBytes},
    zero_indexed_leaf::keccak::ZERO_INDEXED_LEAF,
    Hash, Hasher,
};

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Keccak;

impl Hasher for Keccak {
    fn hash(val: &[u8]) -> Result<Hash, HasherError> {
        Ok(hash(val).to_bytes())
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, HasherError> {
        Ok(hashv(vals).to_bytes())
    }

    fn zero_bytes() -> ZeroBytes {
        ZERO_BYTES
    }

    fn zero_indexed_leaf() -> [u8; 32] {
        ZERO_INDEXED_LEAF
    }
}
