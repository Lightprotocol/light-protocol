//! Zero-copy account state for AccountLoader demonstration.

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use light_account::{CompressionInfo, Discriminator, LightDiscriminator, LightHasherSha};

/// Zero-copy account for demonstrating AccountLoader integration.
///
/// Requirements:
/// - `#[repr(C)]` for predictable field layout
/// - `Pod + Zeroable` (bytemuck) for on-chain zero-copy access
/// - `AnchorSerialize + AnchorDeserialize` for hashing (same as Borsh accounts)
/// - `Discriminator` for dispatch (matches Anchor's `#[account(zero_copy)]`)
/// - compression_info field for rent tracking
/// - All fields must be Pod-compatible (no Pubkey, use [u8; 32])
#[derive(
    Default,
    Debug,
    BorshSerialize,
    BorshDeserialize, // For hashing (same as Borsh accounts)
    Discriminator,    // Must use Anchor discriminator since #[account(zero_copy)] is used
    LightHasherSha,   // For Light Protocol
)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    /// Compression info for rent tracking (must be first for consistent packing).
    /// SDK CompressionInfo is 24 bytes, Pod-compatible.
    pub compression_info: CompressionInfo,
    /// Owner of the record (use byte array instead of Pubkey for Pod compatibility).
    pub owner: [u8; 32],
    /// A value field for demonstration.
    pub value: u64,
}

impl ZeroCopyRecord {
    /// Space required for this account (excluding Anchor discriminator).
    /// compression_info (24) + owner (32) + value (8) = 64 bytes
    pub const INIT_SPACE: usize = core::mem::size_of::<Self>();
}
