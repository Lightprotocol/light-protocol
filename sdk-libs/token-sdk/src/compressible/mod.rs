//! Compressible token utilities for runtime compression and decompression.

mod compress_runtime;
// mod decompress_runtime;
mod mint_runtime;

pub use compress_runtime::*;
// pub use decompress_runtime::*;
pub use mint_runtime::*;
use solana_account_info::AccountInfo;

#[derive(Debug, Clone)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}
