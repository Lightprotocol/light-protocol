//! # light-sdk-types
//!
//! Core types for the Light Protocol SDK.
//!
//! | Type | Description |
//! |------|-------------|
//! | [`RentSponsor`] | PDA to sponsor rent-exemption of Solana accounts using the Light Token Program |
//! | [`CpiAccounts`](cpi_accounts::CpiAccounts) | Container for CPI system and tree accounts |
//! | [`CpiSigner`] | Program ID, signer, and bump for CPI invocation |
//! | [`address`] | Address derivation functions (v1 and v2) |
//! | [`constants`] | Protocol program IDs and discriminators |

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

pub mod address;
pub mod constants;
pub mod cpi_accounts;
pub mod cpi_context_write;
pub mod error;
pub mod instruction;

#[cfg(feature = "std")]
pub mod interface;

// Re-exports
#[cfg(feature = "anchor")]
pub use anchor_lang::{AnchorDeserialize, AnchorSerialize};
#[cfg(not(feature = "anchor"))]
pub use borsh::{BorshDeserialize as AnchorDeserialize, BorshSerialize as AnchorSerialize};
pub use constants::*;
pub use light_account_checks;
pub use light_account_checks::discriminator::Discriminator as LightDiscriminator;
pub use light_compressed_account::CpiSigner;

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct RentSponsor {
    pub program_id: [u8; 32],
    pub rent_sponsor: [u8; 32],
    pub bump: u8,
}
