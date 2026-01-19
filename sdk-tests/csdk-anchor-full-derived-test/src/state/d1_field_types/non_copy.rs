//! D1 Test: Non-Copy field (String) - clone() path
//!
//! Exercises the code path where a non-Copy field (String) exists,
//! which triggers the `.clone()` path in pack/unpack generation.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// A struct with a String field (non-Copy type).
/// This tests the clone() code path for non-Copy fields.
#[derive(Default, Debug, InitSpace, LightAccount)]
#[account]
pub struct NonCopyRecord {
    pub compression_info: Option<CompressionInfo>,
    #[max_len(64)]
    pub name: String,
    #[max_len(128)]
    pub description: String,
    pub counter: u64,
}
