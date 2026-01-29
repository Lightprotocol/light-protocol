//! D11 Test: Zero-copy record with params-only seed fields.
//!
//! Tests `#[light_account(init, zero_copy)]` with params-only seeds (not on struct).

use anchor_lang::prelude::*;
use light_sdk::{interface::CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// Zero-copy record for testing params-only seeds (category_id in seeds but not on struct).
/// The PDA seeds may include params.category_id which is not stored on this struct.
#[derive(Default, Debug, LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZcWithParamsRecord {
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Owner of this record.
    pub owner: Pubkey,
    /// A data value.
    pub data: u64,
}
