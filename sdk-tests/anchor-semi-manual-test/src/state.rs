//! State module for single-pda-derive-test.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// Minimal record struct for testing PDA creation.
/// Contains only compression_info and one field.
#[derive(Default, Debug, PartialEq, InitSpace, LightAccount)]
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// A zero-copy account using Pod serialization.
/// Used with AccountLoader for efficient on-chain zero-copy access.
#[derive(Default, Debug, PartialEq, LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub counter: u64,
}
