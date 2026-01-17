//! D2 Test: compress_as with multiple overrides
//!
//! Exercises the code path where multiple fields have compress_as overrides.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::RentFreeAccount;

/// A struct with multiple compress_as overrides.
/// start, score, and cached all have compression overrides.
#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[compress_as(start = 0, score = 0, cached = 0)]
#[account]
pub struct MultipleCompressAsRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    pub start: u64,
    pub score: u64,
    pub cached: u64,
    pub counter: u64,
}
