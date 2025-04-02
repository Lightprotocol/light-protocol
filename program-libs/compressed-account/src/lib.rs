#![allow(unexpected_cfgs)]

use std::fmt::Display;

use light_hasher::HasherError;
use thiserror::Error;

pub mod address;
pub mod compressed_account;
pub mod constants;
pub mod discriminators;
pub mod hash_chain;
pub mod indexer_event;
pub mod instruction_data;
pub mod nullifier;
pub mod pubkey;
pub mod tx_hash;

#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_hasher::{
    bigint::bigint_to_be_bytes_array,
    hash_to_field_size::{hash_to_bn254_field_size_be, hashv_to_bn254_field_size_be},
};

#[derive(Debug, Error, PartialEq)]
pub enum CompressedAccountError {
    #[error("Invalid input size, expected at most {0}")]
    InputTooLarge(usize),
    #[error("Invalid chunk size")]
    InvalidChunkSize,
    #[error("Invalid seeds")]
    InvalidSeeds,
    #[error("Invalid rollover thresold")]
    InvalidRolloverThreshold,
    #[error("Invalid input lenght")]
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
            CompressedAccountError::HasherError(e) => u32::from(e),
        }
    }
}

impl From<CompressedAccountError> for solana_program::program_error::ProgramError {
    fn from(e: CompressedAccountError) -> Self {
        solana_program::program_error::ProgramError::Custom(e.into())
    }
}

pub const NULLIFIER_QUEUE_TYPE: u64 = 1;
pub const ADDRESS_QUEUE_TYPE: u64 = 2;
pub const BATCHED_INPUT_QUEUE_TYPE: u64 = 3;
pub const BATCHED_ADDRESS_QUEUE_TYPE: u64 = 4;
pub const BATCHED_OUTPUT_QUEUE_TYPE: u64 = 5;

#[derive(AnchorDeserialize, AnchorSerialize, Debug, PartialEq, Clone, Copy)]
#[repr(u64)]
pub enum QueueType {
    NullifierQueue = NULLIFIER_QUEUE_TYPE,
    AddressQueue = ADDRESS_QUEUE_TYPE,
    BatchedInput = BATCHED_INPUT_QUEUE_TYPE,
    BatchedAddress = BATCHED_ADDRESS_QUEUE_TYPE,
    BatchedOutput = BATCHED_OUTPUT_QUEUE_TYPE,
}

impl From<u64> for QueueType {
    fn from(value: u64) -> Self {
        match value {
            1 => QueueType::NullifierQueue,
            2 => QueueType::AddressQueue,
            3 => QueueType::BatchedInput,
            4 => QueueType::BatchedAddress,
            5 => QueueType::BatchedOutput,
            _ => panic!("Invalid queue type"),
        }
    }
}

pub const STATE_MERKLE_TREE_TYPE: u64 = 1;
pub const ADDRESS_MERKLE_TREE_TYPE: u64 = 2;
pub const BATCHED_STATE_MERKLE_TREE_TYPE: u64 = 3;
pub const BATCHED_ADDRESS_MERKLE_TREE_TYPE: u64 = 4;

#[repr(u64)]
#[derive(Debug, PartialEq, Eq, Clone, Copy, AnchorSerialize, AnchorDeserialize)]
pub enum TreeType {
    State = STATE_MERKLE_TREE_TYPE,
    Address = ADDRESS_MERKLE_TREE_TYPE,
    BatchedState = BATCHED_STATE_MERKLE_TREE_TYPE,
    BatchedAddress = BATCHED_ADDRESS_MERKLE_TREE_TYPE,
}

impl Display for TreeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeType::State => write!(f, "State"),
            TreeType::Address => write!(f, "Address"),
            TreeType::BatchedState => write!(f, "BatchedState"),
            TreeType::BatchedAddress => write!(f, "BatchedAddress"),
        }
    }
}

#[allow(clippy::derivable_impls)]
impl std::default::Default for TreeType {
    fn default() -> Self {
        TreeType::BatchedState
    }
}

// from u64
impl From<u64> for TreeType {
    fn from(value: u64) -> Self {
        match value {
            1 => TreeType::State,
            2 => TreeType::Address,
            3 => TreeType::BatchedState,
            4 => TreeType::BatchedAddress,
            _ => panic!("Invalid TreeType"),
        }
    }
}
