//! # Light Compressed Token SDK
//!
//! Low-level SDK for compressed token operations on Light Protocol.
//!
//! This crate provides the core building blocks for working with compressed token accounts,
//! including instruction builders for transfers, mints, and compress/decompress operations.
//!
//! ## Features
//!
//! - `v1` - Enable v1 compressed token support
//! - `anchor` - Enable Anchor framework integration
//! - `anchor-discriminator` - Use Anchor-style discriminators (default)
//!
//! ## Modules
//!
//! - [`compressed_token`] - Core compressed token types and instruction builders
//! - [`error`] - Error types for compressed token operations
//! - [`utils`] - Utility functions and default account configurations
//! - [`constants`] - Program IDs and other constants
//! - [`spl_interface`] - SPL interface PDA derivation utilities

pub mod compat;
pub mod compressed_token;
pub mod constants;
pub mod error;
pub mod spl_interface;
pub mod utils;

// Conditional anchor re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use light_compressed_account::instruction_data::compressed_proof::{
    CompressedProof, ValidityProof,
};
