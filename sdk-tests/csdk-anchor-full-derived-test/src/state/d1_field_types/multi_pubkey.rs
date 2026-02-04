//! D1 Test: Multiple Pubkey fields - PackedX with multiple u8 indices
//!
//! Exercises the code path where 3+ Pubkey fields exist,
//! generating a PackedX struct with multiple u8 index fields.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// A struct with multiple Pubkey fields.
/// PackedMultiPubkeyRecord will have: owner_index, delegate_index, authority_index: u8
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct MultiPubkeyRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub authority: Pubkey,
    pub amount: u64,
}
