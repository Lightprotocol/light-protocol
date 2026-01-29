//! # light-token-interface
//!
//! Instruction data types for the light-token program.
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`instructions`] | Instruction structs for mint, transfer, wrap, unwrap |
//! | [`state`] | Token account and mint state structs |
//! | [`discriminator`] | Instruction discriminator constants |
//! | [`hash_cache`] | Precomputed hashes for token account fields |
//! | [`pool_derivation`] | SPL/T22 pool account PDA derivation |
//! | [`token_2022_extensions`] | Token-2022 extension data types |
//! | [`error`] | `TokenInterfaceError` variants |

pub mod discriminator;
pub mod instructions;

pub mod error;
pub mod hash_cache;
pub mod pool_derivation;
pub mod token_2022_extensions;

pub use error::*;
pub use pool_derivation::*;
pub use token_2022_extensions::*;
mod constants;
pub mod state;
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
