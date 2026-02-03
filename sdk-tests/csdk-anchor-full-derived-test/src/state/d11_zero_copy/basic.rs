//! D11 Test: Basic zero-copy record without complex seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with a simple Pod account.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// Basic zero-copy record for simple tests (no Pubkey seeds on the struct itself).
/// Used with AccountLoader<'info, ZcBasicRecord>.
#[derive(Default, Debug, LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcBasicRecord {
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Owner of this record.
    pub owner: Pubkey,
    /// A simple counter value.
    pub counter: u64,
}

impl PartialEq for ZcBasicRecord {
    fn eq(&self, other: &Self) -> bool {
        self.compression_info == other.compression_info
            && self.owner == other.owner
            && self.counter == other.counter
    }
}
