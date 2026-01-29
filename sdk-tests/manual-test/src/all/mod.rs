//! All account types creation - manual implementation of macro-generated code.
//!
//! This module demonstrates creating ALL account types in a single instruction:
//! - Borsh PDA (MinimalRecord)
//! - ZeroCopy PDA (ZeroCopyRecord)
//! - Compressed Mint
//! - Token Vault
//! - Associated Token Account (ATA)
//!
//! Key pattern: PDAs + Mints require CPI context flow:
//! - PDAs call `invoke_write_to_cpi_context_first()` (writes to CPI context, doesn't execute)
//! - Mints call `invoke_create_mints()` with `.with_cpi_context_offset(NUM_LIGHT_PDAS)` (executes combined CPI)
//! - Token vault and ATA are separate CPIs (don't participate in CPI context)

pub mod accounts;
mod derived;
pub mod derived_accounts;

pub use accounts::*;
pub use derived_accounts::{
    AllBorshSeeds, AllBorshVariant, AllZeroCopySeeds, AllZeroCopyVariant, PackedAllBorshSeeds,
    PackedAllBorshVariant, PackedAllZeroCopySeeds, PackedAllZeroCopyVariant,
};
