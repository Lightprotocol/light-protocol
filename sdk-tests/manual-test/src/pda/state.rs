//! State module for single-pda-test.

use anchor_lang::prelude::*;
use light_sdk::{compressible::CompressionInfo, LightDiscriminator, LightHasherSha};

// ============================================================================
// MinimalRecord with derive macros
// ============================================================================

/// Minimal record struct for testing PDA creation.
/// Contains only compression_info and one field.
///
/// Note: #[account] already derives Clone, AnchorSerialize, AnchorDeserialize
#[derive(Default, Debug, InitSpace, LightDiscriminator, LightHasherSha)]
#[account]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}
