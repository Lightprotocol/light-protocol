//! Compressible token utilities for runtime compression and decompression.

mod mint_runtime;

pub use mint_runtime::*;
use solana_account_info::AccountInfo;

#[derive(Debug, Clone)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}
