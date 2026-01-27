//! D2 Test: compress_as with None value for Option fields
//!
//! Exercises the code path where Option fields are compressed as None.

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// A struct with compress_as None for Option fields.
/// end_time is compressed as None instead of self.end_time.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[compress_as(end_time = None)]
#[account]
pub struct OptionNoneCompressAsRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub start_time: u64,
    pub end_time: Option<u64>,
    pub counter: u64,
}
