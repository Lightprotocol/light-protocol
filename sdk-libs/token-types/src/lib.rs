//! # light-token-types
//!
//! Instruction data and account metadata types for compressed tokens.
//!
//! | Type | Description |
//! |------|-------------|
//! | [`TokenAccountMeta`](instruction::TokenAccountMeta) | Compressed token account metadata |
//! | [`BatchCompressInstructionData`](instruction::BatchCompressInstructionData) | Batch compress instruction data |
//! | [`CompressedTokenInstructionDataApprove`](instruction::CompressedTokenInstructionDataApprove) | Approve/delegation instruction data |
//! | [`PackedMerkleContext`](instruction::PackedMerkleContext) | Merkle tree context for proofs |
//! | [`DelegatedTransfer`](instruction::DelegatedTransfer) | Transfer with delegate as signer |

pub mod account_infos;
pub mod constants;
pub mod error;
pub mod instruction;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use instruction::*;
