//! D4 Test: Large struct with many fields
//!
//! Exercises the hash mode selection for large structs (SHA256 path).

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// Large struct with 12+ fields for SHA256 hash mode.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct LargeRecord {
    pub compression_info: CompressionInfo,
    pub field_01: u64,
    pub field_02: u64,
    pub field_03: u64,
    pub field_04: u64,
    pub field_05: u64,
    pub field_06: u64,
    pub field_07: u64,
    pub field_08: u64,
    pub field_09: u64,
    pub field_10: u64,
    pub field_11: u64,
    pub field_12: u64,
}
