#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

pub mod address;
pub mod constants;
pub mod cpi_accounts;
#[cfg(feature = "cpi-context")]
pub mod cpi_context_write;
pub mod error;
pub mod instruction;

// Re-exports
#[cfg(feature = "anchor")]
use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use light_compressed_account::CpiSigner;
