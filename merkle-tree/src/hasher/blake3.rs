use anchor_lang::solana_program::blake3::{hash, hashv};

use crate::{Hash, Hasher};

#[derive(Clone, Copy)] // To allow using with zero copy Solana accounts.
pub struct Blake3;

impl Hasher for Blake3 {
    fn hash(val: &[u8]) -> Hash {
        hash(val).to_bytes()
    }

    fn hashv(vals: &[&[u8]]) -> Hash {
        hashv(vals).to_bytes()
    }
}
