#![allow(unexpected_cfgs)]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc;

#[cfg(not(feature = "std"))]
pub use alloc::{vec, vec::Vec};
use core::fmt::Display;
#[cfg(feature = "std")]
pub use std::{vec, vec::Vec};

use light_hasher::HasherError;
use thiserror::Error;

pub mod address;
pub mod compressed_account;
pub mod constants;
pub mod discriminators;
pub use light_hasher::hash_chain;
pub mod instruction_data;
pub mod nullifier;
pub mod pubkey;
pub mod tx_hash;
pub use instruction_data::traits::{InstructionDiscriminator, LightInstructionData};
pub use light_hasher::bigint::bigint_to_be_bytes_array;
#[cfg(feature = "alloc")]
pub use light_hasher::hash_to_field_size::{
    hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be,
};
pub use pubkey::Pubkey;

#[derive(Debug, Error, PartialEq)]
pub enum CompressedAccountError {
    #[error("Invalid input size, expected at most {0}")]
    InputTooLarge(usize),
    #[error("Invalid chunk size")]
    InvalidChunkSize,
    #[error("Invalid seeds")]
    InvalidSeeds,
    #[error("Invalid rollover threshold")]
    InvalidRolloverThreshold,
    #[error("Invalid input length")]
    InvalidInputLength,
    #[error("Hasher error {0}")]
    HasherError(#[from] HasherError),
    #[error("Invalid Account size.")]
    InvalidAccountSize,
    #[error("Account is mutable.")]
    AccountMutable,
    #[error("Account is already initialized.")]
    AlreadyInitialized,
    #[error("Invalid account balance.")]
    InvalidAccountBalance,
    #[error("Failed to borrow rent sysvar.")]
    FailedBorrowRentSysvar,
    #[error("Derive address error.")]
    DeriveAddressError,
    #[error("Invalid argument.")]
    InvalidArgument,
    #[error("Expected address for compressed account got None.")]
    ZeroCopyExpectedAddress,
    #[error("Expected address for compressed account got None.")]
    InstructionDataExpectedAddress,
    #[error("Compressed account data not initialized.")]
    CompressedAccountDataNotInitialized,
    #[error(
        "Invalid CPI context configuration: cannot write to CPI context without valid context"
    )]
    InvalidCpiContext,
    #[error("Expected discriminator for compressed account got None.")]
    ExpectedDiscriminator,
    #[error("Expected data hash for compressed account got None.")]
    ExpectedDataHash,
    #[error("Expected proof for compressed account got None.")]
    InstructionDataExpectedProof,
    #[error("Expected proof for compressed account got None.")]
    ZeroCopyExpectedProof,
    #[error("Invalid proof size: expected 128 bytes, got {0}")]
    InvalidProofSize(usize),
}

// NOTE(vadorovsky): Unfortunately, we need to do it by hand.
// `num_derive::ToPrimitive` doesn't support data-carrying enums.
impl From<CompressedAccountError> for u32 {
    fn from(e: CompressedAccountError) -> u32 {
        match e {
            CompressedAccountError::InputTooLarge(_) => 12001,
            CompressedAccountError::InvalidChunkSize => 12002,
            CompressedAccountError::InvalidSeeds => 12003,
            CompressedAccountError::InvalidRolloverThreshold => 12004,
            CompressedAccountError::InvalidInputLength => 12005,
            CompressedAccountError::InvalidAccountSize => 12010,
            CompressedAccountError::AccountMutable => 12011,
            CompressedAccountError::AlreadyInitialized => 12012,
            CompressedAccountError::InvalidAccountBalance => 12013,
            CompressedAccountError::FailedBorrowRentSysvar => 12014,
            CompressedAccountError::DeriveAddressError => 12015,
            CompressedAccountError::InvalidArgument => 12016,
            CompressedAccountError::ZeroCopyExpectedAddress => 12017,
            CompressedAccountError::InstructionDataExpectedAddress => 12018,
            CompressedAccountError::CompressedAccountDataNotInitialized => 12019,
            CompressedAccountError::ExpectedDiscriminator => 12020,
            CompressedAccountError::InstructionDataExpectedProof => 12021,
            CompressedAccountError::ZeroCopyExpectedProof => 12022,
            CompressedAccountError::ExpectedDataHash => 12023,
            CompressedAccountError::InvalidCpiContext => 12024,
            CompressedAccountError::InvalidProofSize(_) => 12025,
            CompressedAccountError::HasherError(e) => u32::from(e),
        }
    }
}

#[cfg(feature = "solana")]
impl From<CompressedAccountError> for solana_program_error::ProgramError {
    fn from(e: CompressedAccountError) -> Self {
        solana_program_error::ProgramError::Custom(e.into())
    }
}

#[cfg(feature = "pinocchio")]
impl From<CompressedAccountError> for pinocchio::program_error::ProgramError {
    fn from(e: CompressedAccountError) -> Self {
        pinocchio::program_error::ProgramError::Custom(e.into())
    }
}

pub const NULLIFIER_QUEUE_TYPE_V1: u64 = 1;
pub const ADDRESS_QUEUE_TYPE_V1: u64 = 2;
pub const INPUT_STATE_QUEUE_TYPE_V2: u64 = 3;
pub const ADDRESS_QUEUE_TYPE_V2: u64 = 4;
pub const OUTPUT_STATE_QUEUE_TYPE_V2: u64 = 5;

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, PartialEq, Clone, Copy)]
#[repr(u64)]
pub enum QueueType {
    NullifierV1 = NULLIFIER_QUEUE_TYPE_V1,
    AddressV1 = ADDRESS_QUEUE_TYPE_V1,
    InputStateV2 = INPUT_STATE_QUEUE_TYPE_V2,
    AddressV2 = ADDRESS_QUEUE_TYPE_V2,
    OutputStateV2 = OUTPUT_STATE_QUEUE_TYPE_V2,
}

impl From<u64> for QueueType {
    fn from(value: u64) -> Self {
        match value {
            1 => QueueType::NullifierV1,
            2 => QueueType::AddressV1,
            3 => QueueType::InputStateV2,
            4 => QueueType::AddressV2,
            5 => QueueType::OutputStateV2,
            _ => panic!("Invalid queue type"),
        }
    }
}

pub const STATE_MERKLE_TREE_TYPE_V1: u64 = 1;
pub const ADDRESS_MERKLE_TREE_TYPE_V1: u64 = 2;
pub const STATE_MERKLE_TREE_TYPE_V2: u64 = 3;
pub const ADDRESS_MERKLE_TREE_TYPE_V2: u64 = 4;

#[cfg_attr(
    all(feature = "std", feature = "anchor"),
    derive(anchor_lang::AnchorDeserialize, anchor_lang::AnchorSerialize)
)]
#[cfg_attr(
    not(feature = "anchor"),
    derive(borsh::BorshDeserialize, borsh::BorshSerialize)
)]
#[derive(Debug, Ord, PartialEq, PartialOrd, Eq, Clone, Copy)]
#[repr(u64)]
pub enum TreeType {
    StateV1 = STATE_MERKLE_TREE_TYPE_V1,
    AddressV1 = ADDRESS_MERKLE_TREE_TYPE_V1,
    StateV2 = STATE_MERKLE_TREE_TYPE_V2,
    AddressV2 = ADDRESS_MERKLE_TREE_TYPE_V2,
}

impl Display for TreeType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TreeType::StateV1 => write!(f, "StateV1"),
            TreeType::AddressV1 => write!(f, "AddressV1"),
            TreeType::StateV2 => write!(f, "StateV2"),
            TreeType::AddressV2 => write!(f, "AddressV2"),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl core::default::Default for TreeType {
    fn default() -> Self {
        TreeType::StateV2
    }
}

// from u64
impl From<u64> for TreeType {
    fn from(value: u64) -> Self {
        match value {
            1 => TreeType::StateV1,
            2 => TreeType::AddressV1,
            3 => TreeType::StateV2,
            4 => TreeType::AddressV2,
            _ => panic!("Invalid TreeType"),
        }
    }
}

/// Configuration struct containing program ID, CPI signer, and bump for Light Protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpiSigner {
    pub program_id: [u8; 32],
    pub cpi_signer: [u8; 32],
    pub bump: u8,
}
