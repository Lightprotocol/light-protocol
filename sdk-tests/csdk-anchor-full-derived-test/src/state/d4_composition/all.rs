//! D4 Test: ALL composition variations combined
//!
//! Exercises a large struct with all field type variants from D1.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// Comprehensive large struct with all field types.
/// 15+ fields to trigger SHA256 mode with all D1 variations.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[compress_as(cached_time = 0, end_time = None)]
#[account]
pub struct AllCompositionRecord {
    pub compression_info: Option<CompressionInfo>,
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub authority: Pubkey,
    pub close_authority: Option<Pubkey>,
    #[max_len(64)]
    pub name: String,
    pub hash: [u8; 32],
    pub start_time: u64,
    pub cached_time: u64,
    pub end_time: Option<u64>,
    pub counter_1: u64,
    pub counter_2: u64,
    pub counter_3: u64,
    pub flag_1: bool,
    pub flag_2: bool,
    pub score: Option<u32>,
}
