//! State module for single-pda-test.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// Minimal record struct for testing PDA creation.
/// Contains only compression_info and one field.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}
