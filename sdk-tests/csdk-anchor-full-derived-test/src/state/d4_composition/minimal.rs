//! D4 Test: Minimal valid struct
//!
//! Exercises the smallest valid struct with compression_info and one field.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::RentFreeAccount;

/// Smallest valid struct: compression_info + one field.
#[derive(Default, Debug, InitSpace, RentFreeAccount)]
#[account]
pub struct MinimalRecord {
    pub compression_info: Option<CompressionInfo>,
    pub value: u64,
}
