//! Compressible token utilities for runtime decompression.

mod decompress_runtime;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use decompress_runtime::*;
use solana_account_info::AccountInfo;

/// Account info with signer seeds for compression operations.
#[derive(Debug, Clone, AnchorSerialize, AnchorDeserialize)]
pub struct AccountInfoToCompress<'info> {
    pub account_info: AccountInfo<'info>,
    pub signer_seeds: Vec<Vec<u8>>,
}
