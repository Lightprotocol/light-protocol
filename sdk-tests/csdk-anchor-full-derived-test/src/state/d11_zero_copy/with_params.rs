//! D11 Test: Zero-copy record with params-only seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with params-only seeds (not on struct).

use anchor_lang::prelude::*;
use light_sdk::interface::CompressionInfo;
use light_sdk::LightDiscriminator;
use light_sdk_macros::PodCompressionInfoField;

/// Zero-copy record for testing params-only seeds (category_id in seeds but not on struct).
/// The PDA seeds may include params.category_id which is not stored on this struct.
#[derive(PodCompressionInfoField)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcWithParamsRecord {
    /// Owner of this record (stored as bytes for Pod compatibility).
    pub owner: [u8; 32],
    /// A data value.
    pub data: u64,
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
}

impl LightDiscriminator for ZcWithParamsRecord {
    // sha256("account:ZcWithParamsRecord")[0..8]
    const LIGHT_DISCRIMINATOR: [u8; 8] = [0x6b, 0xae, 0x5f, 0x90, 0x4d, 0x3c, 0xb1, 0x8e];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl Default for ZcWithParamsRecord {
    fn default() -> Self {
        Self {
            owner: [0u8; 32],
            data: 0,
            compression_info: CompressionInfo::default(),
        }
    }
}
