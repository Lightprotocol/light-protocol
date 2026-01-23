//! Compressible token utilities for runtime decompression.

mod decompress_runtime;

pub use decompress_runtime::*;
use solana_account_info::AccountInfo;

#[derive(Debug, Clone)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}
