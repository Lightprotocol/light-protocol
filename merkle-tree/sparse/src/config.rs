use anchor_lang::prelude::*;

pub trait MerkleTreeConfig {
    const PROGRAM_ID: Pubkey;
}
