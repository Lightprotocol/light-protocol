//! D1 Test: Array fields - [u8; 32], [u8; 8]
//!
//! Exercises the code path for array field handling.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// A struct with array fields.
/// Tests [u8; 32] (byte array) and fixed-size arrays.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct ArrayRecord {
    pub compression_info: CompressionInfo,
    pub hash: [u8; 32],
    pub short_data: [u8; 8],
    pub counter: u64,
}
