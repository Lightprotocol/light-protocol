use anchor_lang::prelude::*;

use crate::constants::ZeroBytes;

pub trait MerkleTreeConfig {
    const ZERO_BYTES: ZeroBytes;
    const PROGRAM_ID: Pubkey;
}
