//! D4 Test: compression_info as last field
//!
//! Exercises struct validation with compression_info in non-first position.

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// Struct with compression_info as last field.
/// Tests that field ordering is handled correctly.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct InfoLastRecord {
    pub owner: Pubkey,
    pub counter: u64,
    pub flag: bool,
    pub compression_info: CompressionInfo,
}
