use anchor_lang::prelude::*;

use light_zero_bytes::ZeroBytes;

pub trait MerkleTreeConfig {
    const ZERO_BYTES: ZeroBytes;
    const PROGRAM_ID: Pubkey;
}
