//! State module for single-pda-test.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator, LightHasherSha};

// ============================================================================
// MinimalRecord with derive macros
// ============================================================================

/// Minimal record struct for testing PDA creation.
/// Contains only compression_info and one field.
///
#[derive(Default, Debug, LightDiscriminator, LightHasherSha)] // LightAccount
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

impl anchor_lang::Space for MinimalRecord {
    const INIT_SPACE: usize = core::mem::size_of::<CompressionInfo>() + 32;
}
