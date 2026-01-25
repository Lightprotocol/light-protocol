//! D11 Test: Zero-copy record with ctx.accounts.* seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with context account seeds.

use anchor_lang::prelude::*;
use light_sdk::interface::CompressionInfo;
use light_sdk::LightDiscriminator;
use light_sdk_macros::PodCompressionInfoField;

/// Zero-copy record with authority field for testing ctx.accounts.* seed packing.
/// The authority field will be used in PDA seeds derived from ctx.accounts.authority.
#[derive(PodCompressionInfoField)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcWithSeedsRecord {
    /// Owner of this record (stored as bytes for Pod compatibility).
    pub owner: [u8; 32],
    /// Authority that controls this record (used as ctx seed).
    pub authority: [u8; 32],
    /// A value field.
    pub value: u64,
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
}

impl LightDiscriminator for ZcWithSeedsRecord {
    // sha256("account:ZcWithSeedsRecord")[0..8]
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0x5a, 0x9d, 0x4e, 0x8f, 0x3c, 0x2b, 0xa0, 0x7d];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl Default for ZcWithSeedsRecord {
    fn default() -> Self {
        Self {
            owner: [0u8; 32],
            authority: [0u8; 32],
            value: 0,
            compression_info: CompressionInfo::default(),
        }
    }
}
