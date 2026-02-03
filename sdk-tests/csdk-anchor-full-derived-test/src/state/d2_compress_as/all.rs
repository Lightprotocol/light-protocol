//! D2 Test: ALL compress_as variations combined
//!
//! Exercises all compress_as code paths in a single struct:
//! - Multiple literal overrides (0)
//! - Option field override (None)
//! - Fields without override (use self.field)

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightAccount, LightDiscriminator};

/// Comprehensive struct with all compress_as variations.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[compress_as(time = 0, end = None, score = 0, cached = 0)]
#[account]
pub struct AllCompressAsRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    // Override with 0
    pub time: u64,
    pub score: u64,
    pub cached: u64,
    // Override with None
    pub end: Option<u64>,
    // No override - uses self.field
    pub counter: u64,
    pub flag: bool,
}
