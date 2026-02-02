//! D1 Test: Option<Pubkey> field
//!
//! Exercises the code path where Option<Pubkey> fields exist,
//! which generates Option<u8> in the packed struct.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// A struct with Option<Pubkey> fields.
/// PackedOptionPubkeyRecord will have: delegate_index: Option<u8>
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct OptionPubkeyRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub delegate: Option<Pubkey>,
    pub close_authority: Option<Pubkey>,
    pub amount: u64,
}
