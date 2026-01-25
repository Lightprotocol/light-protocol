//! D11 Test: Basic zero-copy record without complex seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with a simple Pod account.

use anchor_lang::prelude::*;
use light_sdk::interface::CompressionInfo;
use light_sdk::LightDiscriminator;
use light_sdk_macros::PodCompressionInfoField;

/// Basic zero-copy record for simple tests (no Pubkey seeds on the struct itself).
/// Used with AccountLoader<'info, ZcBasicRecord>.
#[derive(PodCompressionInfoField)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcBasicRecord {
    /// Owner of this record (stored as bytes for Pod compatibility).
    pub owner: [u8; 32],
    /// A simple counter value.
    pub counter: u64,
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
}

impl LightDiscriminator for ZcBasicRecord {
    // sha256("account:ZcBasicRecord")[0..8]
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0x4f, 0x8c, 0x3d, 0x7e, 0x2b, 0x1a, 0x9f, 0x6c];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl Default for ZcBasicRecord {
    fn default() -> Self {
        Self {
            owner: [0u8; 32],
            counter: 0,
            compression_info: CompressionInfo::default(),
        }
    }
}
