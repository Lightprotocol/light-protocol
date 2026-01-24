//! State module for single-account-loader-test.
//!
//! Defines a Pod (zero-copy) account struct for testing AccountLoader with Light Protocol.

use anchor_lang::prelude::*;
use light_sdk::interface::CompressionInfo; // SDK version (24 bytes, Pod-compatible)
use light_sdk::LightDiscriminator;
use light_sdk_macros::PodCompressionInfoField;

/// A zero-copy account using Pod serialization.
/// This account is used with AccountLoader and requires `#[light_account(init, zero_copy)]`.
///
/// Key differences from Borsh-serialized accounts:
/// - Uses `#[repr(C)]` for predictable memory layout
/// - Implements `Pod` + `Zeroable` from bytemuck
/// - Uses non-optional SDK `CompressionInfo` (24 bytes, state indicated by `state` field)
/// - Fixed size at compile time via `core::mem::size_of::<T>()`
#[derive(PodCompressionInfoField)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    /// Owner of this record (stored as bytes for Pod compatibility).
    pub owner: [u8; 32],
    /// A simple counter value.
    pub counter: u64,
    /// Compression state - required for all rent-free accounts.
    /// Uses SDK CompressionInfo (24 bytes):
    /// - `state == Uninitialized` means not yet set up
    /// - `state == Decompressed` means initialized/decompressed
    /// - `state == Compressed` means compressed
    pub compression_info: CompressionInfo,
}

impl LightDiscriminator for ZeroCopyRecord {
    // Must match Anchor's discriminator: sha256("account:ZeroCopyRecord")[0..8]
    // This is computed by Anchor's #[account(zero_copy)] attribute
    const LIGHT_DISCRIMINATOR: [u8; 8] = [55, 26, 139, 203, 102, 125, 85, 82];
    const LIGHT_DISCRIMINATOR_SLICE: &'static [u8] = &Self::LIGHT_DISCRIMINATOR;
}

impl Default for ZeroCopyRecord {
    fn default() -> Self {
        Self {
            owner: [0u8; 32],
            counter: 0,
            compression_info: CompressionInfo::default(),
        }
    }
}
