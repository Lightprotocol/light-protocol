//! State module for single-account-loader-test.
//!
//! Defines a Pod (zero-copy) account struct for testing AccountLoader with Light Protocol.

use anchor_lang::prelude::*;
use light_account::{CompressionInfo, LightDiscriminator};
use light_sdk_macros::LightAccount;

/// A zero-copy account using Pod serialization.
/// This account is used with AccountLoader and requires `#[light_account(init, zero_copy)]`.
///
/// Requirements for zero-copy accounts:
/// - `#[repr(C)]` for predictable memory layout
/// - `Pod + Zeroable` (bytemuck) for on-chain zero-copy access
/// - `LightAccount` derive handles: LightDiscriminator, LightHasherSha, pack/unpack, compression_info
/// - compression_info field for rent tracking
#[derive(Default, Debug, LightAccount)]
#[account(zero_copy)]
#[repr(C)]
pub struct ZeroCopyRecord {
    /// Compression state - required for all rent-free accounts.
    pub compression_info: CompressionInfo,
    /// Owner of this record.
    pub owner: Pubkey,
    /// A simple counter value.
    pub counter: u64,
}
