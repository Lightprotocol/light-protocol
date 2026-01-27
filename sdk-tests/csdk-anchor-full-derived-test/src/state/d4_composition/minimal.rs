//! D4 Test: Minimal valid struct
//!
//! Exercises the smallest valid struct with compression_info and one field.

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// Smallest valid struct: compression_info + one field.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub value: u64,
}
