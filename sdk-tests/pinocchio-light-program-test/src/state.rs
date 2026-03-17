//! State module for pinocchio-light-program-test.
//!
//! Pinocchio-compatible account types using BorshSerialize/BorshDeserialize
//! instead of Anchor's #[account] macro.

use borsh::{BorshDeserialize, BorshSerialize};
use light_account_pinocchio::{CompressionInfo, LightDiscriminator, LightPinocchioAccount};
use pinocchio::pubkey::Pubkey;

/// Minimal record struct for testing PDA creation.
/// Contains compression_info and one field.
///
/// LightPinocchioAccount generates:
/// - LightHasherSha (DataHasher + ToByteArray)
/// - LightDiscriminator
/// - LightAccount trait impl with pack/unpack
/// - PackedMinimalRecord struct
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[repr(C)]
pub struct MinimalRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// A PDA with a 1-byte on-chain type identifier instead of the standard 8-byte
/// LIGHT_DISCRIMINATOR. On-chain layout: `[1 byte DISC][borsh data]`.
///
/// `LIGHT_DISCRIMINATOR = [1,0,0,0,0,0,0,0]` (8 bytes, for the compressed Merkle leaf).
/// `LIGHT_DISCRIMINATOR_SLICE = &[1u8]` (1 byte, written on-chain).
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [1u8])]
#[repr(C)]
pub struct OneByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 2-byte on-chain discriminator. LIGHT_DISCRIMINATOR_SLICE = &[2u8, 2u8].
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [2u8, 2u8])]
#[repr(C)]
pub struct TwoByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 3-byte on-chain discriminator. LIGHT_DISCRIMINATOR_SLICE = &[3u8, 3u8, 3u8].
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [3u8, 3u8, 3u8])]
#[repr(C)]
pub struct ThreeByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 4-byte on-chain discriminator.
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [4u8, 4u8, 4u8, 4u8])]
#[repr(C)]
pub struct FourByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 5-byte on-chain discriminator.
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [5u8, 5u8, 5u8, 5u8, 5u8])]
#[repr(C)]
pub struct FiveByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 6-byte on-chain discriminator.
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [6u8, 6u8, 6u8, 6u8, 6u8, 6u8])]
#[repr(C)]
pub struct SixByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
}

/// 7-byte on-chain discriminator.
#[derive(
    Default, Debug, Clone, PartialEq, BorshSerialize, BorshDeserialize, LightPinocchioAccount,
)]
#[light_pinocchio(discriminator = [7u8, 7u8, 7u8, 7u8, 7u8, 7u8, 7u8])]
#[repr(C)]
pub struct SevenByteRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
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
    LightPinocchioAccount,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
#[repr(C)]
pub struct ZeroCopyRecord {
    pub compression_info: CompressionInfo,
    pub owner: Pubkey,
    pub counter: u64,
}
