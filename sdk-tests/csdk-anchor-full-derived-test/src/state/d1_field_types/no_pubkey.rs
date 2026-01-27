//! D1 Test: No Pubkey fields - Identity Pack generation
//!
//! Exercises the code path where no Pubkey fields exist,
//! resulting in Pack/Unpack being a type alias (identity).

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// A struct with only primitive fields - no Pubkey.
/// This tests the identity Pack path where PackedNoPubkeyRecord = NoPubkeyRecord.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct NoPubkeyRecord {
    pub compression_info: CompressionInfo,
    pub counter: u64,
    pub flag: bool,
    pub value: u32,
}
