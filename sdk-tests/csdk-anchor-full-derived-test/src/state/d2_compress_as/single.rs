//! D2 Test: compress_as with single override
//!
//! Exercises the code path where one field has a compress_as override.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// A struct with single compress_as override.
/// cached field is compressed as 0 instead of self.cached.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[compress_as(cached = 0)]
#[account]
pub struct SingleCompressAsRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    pub cached: u64,
    pub counter: u64,
}
