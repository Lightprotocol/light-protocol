//! D1 Test: Option<primitive> fields
//!
//! Exercises the code path where Option<u64>, Option<bool>, etc. exist.
//! These remain unchanged in the packed struct (not converted to u8 index).

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// A struct with Option<primitive> fields.
/// These stay as Option<T> in the packed struct (not Option<u8>).
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct OptionPrimitiveRecord {
    pub compression_info: CompressionInfo,
    pub counter: u64,
    pub end_time: Option<u64>,
    pub enabled: Option<bool>,
    pub score: Option<u32>,
}
