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

#[derive(Clone, Copy, Debug, PartialEq, Eq, AnchorSerialize, AnchorDeserialize)]
pub struct RentSponsor {
    pub program_id: [u8; 32],
    pub rent_sponsor: [u8; 32],
    pub bump: u8,
    pub version: u16,
}

/// Pre-computed rent sponsor PDAs for versions 1-4.
/// Version 1 is always the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RentSponsors {
    pub sponsors: [RentSponsor; 4],
}

impl RentSponsors {
    /// Returns the default rent sponsor (version 1).
    #[inline]
    pub const fn default(&self) -> &RentSponsor {
        &self.sponsors[0]
    }

    /// Returns the rent sponsor for the given version (1-4).
    /// Returns None if version is 0 or > 4.
    #[inline]
    pub const fn get(&self, version: u16) -> Option<&RentSponsor> {
        if version == 0 || version > 4 {
            None
        } else {
            Some(&self.sponsors[(version - 1) as usize])
        }
    }

    /// Returns all 4 rent sponsors.
    #[inline]
    pub const fn all(&self) -> &[RentSponsor; 4] {
        &self.sponsors
    }
}
