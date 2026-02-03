//! State module for pinocchio-light-program-test.
//!
//! Pinocchio-compatible account types using BorshSerialize/BorshDeserialize
//! instead of Anchor's #[account] macro.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightHasherSha};

/// Minimal record struct for testing PDA creation.
/// Contains compression_info and one field.
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightDiscriminator,
    LightHasherSha,
)]
#[repr(C)]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: [u8; 32],
}

impl MinimalRecord {
    pub const INIT_SPACE: usize = core::mem::size_of::<CompressionInfo>() + 32;
}

/// A zero-copy account using Pod serialization.
/// Used for efficient on-chain zero-copy access.
#[derive(
    Default,
    Debug,
    Copy,
    Clone,
    PartialEq,
    BorshSerialize,
    BorshDeserialize,
    LightDiscriminator,
    LightHasherSha,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
#[repr(C)]
pub struct ZeroCopyRecord {
    pub compression_info: CompressionInfo,
    pub owner: [u8; 32],
    pub counter: u64,
}

impl ZeroCopyRecord {
    pub const INIT_SPACE: usize = core::mem::size_of::<Self>();
}
