#[cfg(feature = "solana")]
use anchor_lang::prelude::*;

use crate::constants::ZeroBytes;

pub trait MerkleTreeConfig {
    const ZERO_BYTES: ZeroBytes;
    #[cfg(feature = "solana")]
    const PROGRAM_ID: Pubkey;
}

#[cfg(not(feature = "solana"))]
mod configs {
    use super::*;

    use crate::constants;

    pub struct Sha256MerkleTreeConfig;

    impl MerkleTreeConfig for Sha256MerkleTreeConfig {
        const ZERO_BYTES: ZeroBytes = constants::sha256::ZERO_BYTES;
    }
}

#[cfg(not(feature = "solana"))]
pub use configs::*;
