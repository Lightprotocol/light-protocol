//! D1 Test: ALL field types combined
//!
//! Exercises all field type code paths in a single struct:
//! - Multiple Pubkeys (-> u8 indices)
//! - Option<Pubkey> (-> Option<u8>)
//! - String (-> clone() path)
//! - Arrays (-> direct copy)
//! - Option<primitives> (-> unchanged)

use anchor_lang::prelude::*;
use light_sdk::compressible::CompressionInfo;
use light_sdk_macros::LightAccount;

/// Comprehensive struct with all field type variations.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct AllFieldTypesRecord {
    pub compression_info: CompressionInfo,
    // Multiple Pubkeys -> _index: u8 fields
    pub owner: Pubkey,
    pub delegate: Pubkey,
    pub authority: Pubkey,
    // Option<Pubkey> -> Option<u8>
    pub close_authority: Option<Pubkey>,
    // String -> clone() path
    #[max_len(64)]
    pub name: String,
    // Arrays -> direct copy
    pub hash: [u8; 32],
    // Option<primitives> -> unchanged
    pub end_time: Option<u64>,
    pub enabled: Option<bool>,
    // Regular primitives
    pub counter: u64,
    pub flag: bool,
}
