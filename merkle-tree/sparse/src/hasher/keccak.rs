use anchor_lang::solana_program::keccak::{hash, hashv};

use crate::{errors::MerkleTreeError, Hash, Hasher};

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Keccak;

impl Hasher for Keccak {
    fn hash(val: &[u8]) -> Result<Hash, MerkleTreeError> {
        Ok(hash(val).to_bytes())
    }

    fn hashv(vals: &[&[u8]]) -> Result<Hash, MerkleTreeError> {
        Ok(hashv(vals).to_bytes())
    }
}
