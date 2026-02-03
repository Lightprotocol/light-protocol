//! D1 Test: Single Pubkey field - PackedX with one u8 index
//!
//! Exercises the code path where exactly one Pubkey field exists,
//! generating a PackedX struct with a single u8 index field.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// A struct with exactly one Pubkey field.
/// PackedSinglePubkeyRecord will have: owner_index: u8
#[derive(Default, Debug, PartialEq, InitSpace, LightAccount)]
#[account]
pub struct SinglePubkeyRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub counter: u64,
}
