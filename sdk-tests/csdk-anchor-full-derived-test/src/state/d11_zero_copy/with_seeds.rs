//! D11 Test: Zero-copy record with ctx.accounts.* seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with context account seeds.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// Zero-copy record with authority field for testing ctx.accounts.* seed packing.
/// The authority field will be used in PDA seeds derived from ctx.accounts.authority.
#[derive(Default, Debug, LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcWithSeedsRecord {
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Owner of this record.
    pub owner: Pubkey,
    /// Authority that controls this record (used as ctx seed).
    pub authority: Pubkey,
    /// A value field.
    pub value: u64,
}

impl PartialEq for ZcWithSeedsRecord {
    fn eq(&self, other: &Self) -> bool {
        self.compression_info == other.compression_info
            && self.owner == other.owner
            && self.authority == other.authority
            && self.value == other.value
    }
}
